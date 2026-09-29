import { useEffect, useRef, useState } from 'react';
import { Check, ChevronDown, File, FolderOpen, HardDrive, LayoutDashboard, LockKeyhole, KeyRound, FilePlus2 } from 'lucide-react';
import { Button } from '@/components/ui/button';
import VaultWorkspace from '@/components/VaultWorkspace';
import VaultFiles from '@/components/VaultFiles';
import VaultFileEditor from '@/components/VaultFileEditor';
import ExampleFiles from '@/components/ExampleFiles';
import AuthGate from '@/components/AuthGate';
import SplashScreen from '@/components/SplashScreen';
import VaultMark from '@/components/VaultMark';
import { DEFAULT_IDLE_MS, hasSessionExpired } from '@/lib/sessionTimeout';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { errorMessage, native, type DesktopInfo, type VaultSummary, type VaultFile } from '@/lib/native';

export default function App() {
  const [info, setInfo] = useState<DesktopInfo | null>(null);
  const [session, setSession] = useState<VaultSummary | null>(null);
  const [booting, setBooting] = useState(true);
  const [examplesOpen, setExamplesOpen] = useState(false);
  const [page, setPage] = useState<'overview' | 'vault' | 'create'>('overview');
  const [storedFiles, setStoredFiles] = useState<VaultFile[]>([]);
  const [decryptedFiles, setDecryptedFiles] = useState<VaultFile[]>([]);
  const [chosenDecrypt, setChosenDecrypt] = useState<{ id: string; nonce: number } | null>(null);
  const [chosenExample, setChosenExample] = useState<{ id: string; nonce: number } | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const [notice, setNotice] = useState('');
  const locking = useRef(false);

  useEffect(() => {
    let active = true;
    // Fail closed: a webview reload must not inherit an unlocked Rust vault session.
    Promise.all([native.lockVault(), native.getDesktopInfo(), new Promise<void>(resolve => setTimeout(resolve, 2800))]).then(([, value]) => {
      if (active) setInfo(value);
    }).catch(reason => { if (active) setError(errorMessage(reason)); })
      .finally(() => { if (active) setBooting(false); });
    return () => { active = false; };
  }, []);

  async function signOut() {
    if (locking.current) return;
    locking.current = true;
    setBusy(true); setError('');
    try {
      await native.lockVault();
      setSession(null); setStoredFiles([]); setDecryptedFiles([]); setNotice('');
    } catch (reason) { setError(errorMessage(reason)); }
    finally { setBusy(false); locking.current = false; }
  }

  useEffect(() => {
    if (!session) return;
    let lastActivity = Date.now();
    const expire = () => {
      if (locking.current || !hasSessionExpired(lastActivity, Date.now())) return;
      void signOut();
    };
    const activity = () => {
      // A delayed event after sleep must not extend an already expired session.
      if (hasSessionExpired(lastActivity, Date.now())) { expire(); return; }
      lastActivity = Date.now();
    };
    const events = ['pointerdown', 'keydown', 'wheel', 'touchstart'] as const;
    for (const event of events) window.addEventListener(event, activity, { passive: true });
    window.addEventListener('focus', expire);
    const interval = window.setInterval(expire, 1000);
    return () => {
      window.clearInterval(interval);
      for (const event of events) window.removeEventListener(event, activity);
      window.removeEventListener('focus', expire);
    };
  }, [session]);

  if (booting) return <SplashScreen />;
  if (!info) return <main className="grid place-items-center p-10"><p role="alert" className="text-sm text-red-300">{error || 'Native desktop unavailable.'}</p></main>;
  if (!session) return <AuthGate onAuthenticated={setSession} />;

  return <div className="grid grid-cols-1 md:grid-cols-[260px_minmax(0,1fr)]">
    <aside className="flex flex-col border-b border-border bg-[#0b1018] p-5 md:border-r md:border-b-0">
      <div className="flex items-center gap-3 py-2">
        <div className="flex size-10 items-center justify-center rounded-xl border border-blue-400/25 bg-blue-500/10"><VaultMark /></div>
        <span className="text-xs font-semibold tracking-[0.12em]">BLACK VAULT</span>
      </div>
      <button type="button" onClick={() => setPage('overview')} aria-current={page === 'overview' ? 'page' : undefined}
        className={"mt-7 flex items-center gap-3 rounded-lg border px-3 py-2.5 text-left text-sm transition-colors " + (page === 'overview' ? 'border-blue-400/10 bg-blue-500/10 text-blue-300' : 'border-transparent text-slate-400 hover:text-blue-300')}>
        <LayoutDashboard size={16} /> Overview
      </button>
      <button type="button" onClick={() => setPage('vault')} aria-current={page === 'vault' ? 'page' : undefined}
        className={"mt-2 flex items-center gap-3 rounded-lg border px-3 py-2.5 text-left text-sm transition-colors " + (page === 'vault' ? 'border-blue-400/10 bg-blue-500/10 text-blue-300' : 'border-transparent text-slate-400 hover:text-blue-300')}>
        <KeyRound size={16} /> My Vault
      </button>
      <button type="button" onClick={() => setPage('create')} aria-current={page === 'create' ? 'page' : undefined}
        className={"mt-2 flex items-center gap-3 rounded-lg border px-3 py-2.5 text-left text-sm transition-colors " + (page === 'create' ? 'border-blue-400/10 bg-blue-500/10 text-blue-300' : 'border-transparent text-slate-400 hover:text-blue-300')}>
        <FilePlus2 size={16} /> Create Files
      </button>
      <button type="button" aria-controls="example-files-panel" aria-expanded={examplesOpen}
        onClick={() => setExamplesOpen(value => !value)}
        className={"mt-2 flex w-full items-center gap-3 rounded-lg border px-3 py-2.5 text-left text-sm transition-colors duration-200 " + (examplesOpen ? "border-blue-400/15 bg-blue-400/[0.07] text-sky-200" : "border-transparent text-slate-400 hover:bg-white/[0.035] hover:text-blue-300")}>
        <FolderOpen size={16} className="shrink-0" /> Example Files
        <ChevronDown size={15} className={"ml-auto transition-transform duration-300 ease-out " + (examplesOpen ? "rotate-180" : "")} />
      </button>
      <div id="example-files-panel" aria-hidden={!examplesOpen} inert={!examplesOpen}
        className={"grid transition-[grid-template-rows,opacity,transform] duration-300 ease-out motion-reduce:transition-none " + (examplesOpen ? "grid-rows-[1fr] translate-y-0 opacity-100" : "pointer-events-none grid-rows-[0fr] -translate-y-1 opacity-0")}>
        <div className="min-h-0 overflow-hidden">
          <ExampleFiles decryptFiles={storedFiles}
            onSelect={id => { setPage('overview'); setChosenExample(current => ({ id, nonce: (current?.nonce ?? 0) + 1 })); }}
            onDecryptSelect={id => { setPage('overview'); setChosenDecrypt(current => ({ id, nonce: (current?.nonce ?? 0) + 1 })); }} />
        </div>
      </div>
      <div className="mt-auto hidden space-y-3 pt-12 md:block">
        <div className="flex items-center gap-2 text-xs text-slate-400"><span className="size-1.5 rounded-full bg-emerald-400" /> On this Mac</div>
        <p className="font-mono text-[10px] uppercase tracking-widest text-slate-600">Desktop preview · {info?.version ?? '—'}</p>
      </div>
    </aside>

    <main className="mx-auto w-full max-w-6xl px-6 py-8 sm:px-10 sm:py-10">
      <header className="flex flex-wrap items-center justify-between gap-3 border-b border-border pb-6">
        <div className="flex items-center gap-2 text-xs text-slate-400"><HardDrive size={14} /> Personal workspace <span className="mx-1 text-slate-700">/</span><span className="text-slate-200">{page === 'overview' ? 'Overview' : page === 'vault' ? 'My Vault' : 'Create Files'}</span></div>
        <Button variant="outline" disabled={busy} onClick={() => void signOut()}><LockKeyhole size={14} />{busy ? 'Signing out…' : 'Sign out'}</Button>
      </header>

      <div className={page === 'overview' ? '' : 'hidden'} aria-hidden={page !== 'overview'} inert={page !== 'overview'}>
      <section className="pt-6 pb-4">
        <p className="mb-3 font-mono text-[10px] uppercase tracking-[0.2em] text-blue-400">Your private workspace</p>
        <h1 className="text-2xl font-medium tracking-tight sm:text-3xl">Encrypt and decrypt your files.</h1>
        <p className="mt-2 max-w-xl text-xs leading-6 text-slate-400">Open Example Files on the left. Click a sample to select it, or drag it into the workspace below.</p>
      </section>

      <section className="rounded-xl border border-border bg-card px-5 pb-6 pt-1 sm:px-7"><VaultFiles vaultId={session.id} chosenExample={chosenExample} chosenDecrypt={chosenDecrypt} onFilesChange={setStoredFiles} onDecryptedChange={setDecryptedFiles} /></section>

      <div className="mt-6 grid gap-4 lg:grid-cols-3">
        <Card className="gap-3 rounded-xl border border-border ring-0">
          <CardHeader><CardTitle className="flex items-center gap-2 text-xs font-normal text-slate-400"><HardDrive size={14} /> Application</CardTitle></CardHeader>
          <CardContent><p className="text-lg">{info ? 'Native desktop' : error ? 'Desktop unavailable' : 'Connecting…'}</p><p className="mt-1 text-xs text-slate-500">{info ? 'Running locally on your device' : error ? 'Launch the desktop application' : 'Checking the desktop connection'}</p></CardContent>
        </Card>
        <Card className="gap-3 rounded-xl border border-border ring-0">
          <CardHeader><CardTitle className="flex items-center gap-2 text-xs font-normal text-slate-400"><LockKeyhole size={14} /> Local vault</CardTitle></CardHeader>
          <CardContent><p className="text-lg">Offline vaults</p><p className="mt-1 text-xs text-slate-500">Create and manage vaults below</p></CardContent>
        </Card>
        <Card className="gap-3 rounded-xl border border-border ring-0">
          <CardHeader><CardTitle className="flex items-center gap-2 text-xs font-normal text-slate-400"><File size={14} /> Current selection</CardTitle></CardHeader>
          <CardContent><p className="text-lg">'Encrypted locally'</p><p className="mt-1 text-xs text-slate-500">'Manage files using the workspace above'</p></CardContent>
        </Card>
      </div>

      </div>
      {page === 'create' && <div className="pt-6">
        <p className="mb-3 font-mono text-[10px] uppercase tracking-[0.2em] text-blue-400">In-vault documents</p>
        <h1 className="text-2xl font-medium tracking-tight sm:text-3xl">Create Files</h1>
        <p className="mt-2 max-w-xl text-xs leading-6 text-slate-400">Write a document and encrypt it directly into your unlocked vault. No external Save dialog or plaintext file is created.</p>
        <VaultFileEditor vaultId={session.id} onFilesChange={setStoredFiles}/>
      </div>}
      {page === 'vault' && <div className="pt-6">
        <p className="mb-3 font-mono text-[10px] uppercase tracking-[0.2em] text-blue-400">Local security</p>
        <h1 className="text-2xl font-medium tracking-tight sm:text-3xl">My Vault</h1>
        <p className="mt-2 max-w-xl text-xs leading-6 text-slate-400">Your encrypted vault, local files, session state and security controls. No cloud connection is required.</p>
        <div className="mt-6 grid gap-4 sm:grid-cols-3">
          <div className="rounded-xl border border-border bg-card p-5"><p className="text-xs text-slate-400">Current vault</p><p className="mt-2 break-all text-sm font-medium">{session.name ?? 'Unlocked vault'}</p></div>
          <div className="rounded-xl border border-border bg-card p-5"><p className="text-xs text-slate-400">Encrypted files</p><p className="mt-2 text-xl font-medium">{storedFiles.length}</p></div>
          <div className="rounded-xl border border-border bg-card p-5"><p className="text-xs text-slate-400">Session security</p><p className="mt-2 text-sm font-medium text-emerald-300">Unlocked · 5-minute auto-lock</p></div>
        </div>
        <section className="mt-6 rounded-xl border border-border bg-card p-5">
          <h2 className="text-sm font-medium">Vault contents</h2>
          <p className="mt-1 text-xs text-slate-400">Encrypted files are retained in the local vault. Decrypted entries below are session-only verification records, not saved plaintext copies.</p>
          <div className="mt-4 grid gap-4 sm:grid-cols-2">
            <div><h3 className="text-xs font-medium text-sky-300">Encrypted vault files · {storedFiles.length}</h3>
              {storedFiles.length === 0 ? <p className="mt-3 text-xs text-slate-500">No encrypted files.</p> : storedFiles.map(file => <p key={file.id} className="mt-2 break-all rounded-lg border border-border p-2 text-xs">{file.name}</p>)}
            </div>
            <div><h3 className="text-xs font-medium text-emerald-300">Decrypted this session · {decryptedFiles.length}</h3>
              {decryptedFiles.length === 0 ? <p className="mt-3 text-xs text-slate-500">Nothing decrypted yet.</p> : decryptedFiles.map(file => <p key={file.id} className="mt-2 break-all rounded-lg border border-border p-2 text-xs">{file.name} · verified in memory</p>)}
            </div>
          </div>
        </section>
        <VaultWorkspace ready={Boolean(info)} onLocked={() => { setStoredFiles([]); setDecryptedFiles([]); setSession(null); }} />
      </div>}

      {error && <p role="alert" className="mt-4 rounded-lg border border-red-400/20 bg-red-400/5 px-4 py-3 text-sm text-red-300">{error}</p>}
      <p role="status" className="mt-4 flex items-center gap-2 text-xs text-slate-400">{notice && <><Check size={14} className="text-emerald-400" />{notice}</>}</p>
      <footer className="mt-8 flex flex-wrap justify-between gap-2 border-t border-border pt-5 font-mono text-[10px] uppercase tracking-widest text-slate-600"><span>Black Vault · Desktop foundation</span><span>{info ? `${info.platform} / ${info.architecture}` : 'Native connection pending'} · Auto-lock after {DEFAULT_IDLE_MS / 60000} min idle</span></footer>
    </main>
  </div>;
}
