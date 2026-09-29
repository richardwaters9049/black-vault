import { useEffect, useRef, useState } from 'react';
import { Check, ChevronDown, File, FolderOpen, HardDrive, LayoutDashboard, LockKeyhole, KeyRound, FilePlus2, Lock, LockOpen } from 'lucide-react';
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
import { errorMessage, native, type DesktopInfo, type VaultSummary, type VaultFile, type NestedVaultSummary } from '@/lib/native';

export default function App() {
  const [info, setInfo] = useState<DesktopInfo | null>(null);
  const [session, setSession] = useState<VaultSummary | null>(null);
  const [booting, setBooting] = useState(true);
  const [examplesOpen, setExamplesOpen] = useState(false);
  const [page, setPage] = useState<'overview' | 'vault' | 'create' | 'nested'>('overview');
  const [storedFiles, setStoredFiles] = useState<VaultFile[]>([]);
  const [decryptedFiles, setDecryptedFiles] = useState<VaultFile[]>([]);
  const [nestedVaults, setNestedVaults] = useState<NestedVaultSummary[]>([]);
  const [selectedNested, setSelectedNested] = useState<string | null>(null);
  const [nestedPassword, setNestedPassword] = useState('');
  const [nestedContents, setNestedContents] = useState<VaultFile[]>([]);
  const [nestedError, setNestedError] = useState('');
  const [nestedBusy, setNestedBusy] = useState(false);
  const [nestedUnlocked, setNestedUnlocked] = useState(false);
  const [nestedFileName, setNestedFileName] = useState('');
  const [nestedFileType, setNestedFileType] = useState('txt');
  const [nestedFileContent, setNestedFileContent] = useState('');
  const [lockIds, setLockIds] = useState<string[]>([]);
  const [lockName, setLockName] = useState('');
  const [lockPassword, setLockPassword] = useState('');
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

  useEffect(() => {
    if (!session) { setNestedVaults([]); setSelectedNested(null); setNestedContents([]); setNestedPassword(''); setNestedUnlocked(false); setNestedFileContent(''); return; }
    let active = true;
    native.listNestedVaults().then(items => { if (active) setNestedVaults(items); })
      .catch(reason => { if (active) setNestedError(errorMessage(reason)); });
    return () => { active = false; };
  }, [session]);

  async function lockFiles(event: React.FormEvent) {
    event.preventDefault();
    if (nestedBusy || !lockIds.length || !/^[0-9]{4}$/.test(lockPassword)) {
      setNestedError('Choose exactly four digits (0–9) for the nested vault PIN.'); return;
    }
    setNestedBusy(true); setNestedError('');
    try {
      await native.lockSelectedFiles(lockName.trim(), lockPassword, lockIds);
      const [files, nested] = await Promise.all([native.listVaultFiles(), native.listNestedVaults()]);
      setStoredFiles(files); setNestedVaults(nested); setLockIds([]); setLockName(''); setLockPassword('');
      setChosenDecrypt(null); setDecryptedFiles([]);
      setNotice('Selected files secured in a nested vault with your four-digit PIN.');
    } catch (reason) { setNestedError(errorMessage(reason)); }
    finally { setLockPassword(''); setNestedBusy(false); }
  }

  async function restoreNested() {
    if (!selectedNested || nestedBusy || !nestedPassword) return;
    setNestedBusy(true); setNestedError('');
    try {
      await native.restoreNestedVault(selectedNested, nestedPassword);
      const [files, nested] = await Promise.all([native.listVaultFiles(), native.listNestedVaults()]);
      setStoredFiles(files); setNestedVaults(nested); setNestedContents([]); setNestedUnlocked(false); setNestedFileContent(''); setSelectedNested(null);
      setNotice('Nested vault restored into the parent vault as encrypted files.');
    } catch (reason) { setNestedError(errorMessage(reason)); }
    finally { setNestedPassword(''); setNestedBusy(false); }
  }

  async function unlockNested(event: React.FormEvent) {
    event.preventDefault();
    if (!selectedNested || nestedBusy || !nestedPassword) return;
    setNestedBusy(true); setNestedError(''); setNestedContents([]); setNestedUnlocked(false);
    try {
      const contents = await native.unlockNestedVault(selectedNested, nestedPassword);
      setNestedContents(contents); setNestedUnlocked(true); setNestedPassword('');
      setNotice('Nested vault authenticated. Its files remain sealed inside the nested vault.');
    } catch (reason) { setNestedError(errorMessage(reason)); }
    finally { setNestedPassword(''); setNestedBusy(false); }
  }

  async function createNestedFile(event: React.FormEvent) {
    event.preventDefault();
    if (!selectedNested || nestedBusy || !nestedUnlocked || !/^[0-9]{4}$/.test(nestedPassword)) return;
    const name = nestedFileName.trim() + '.' + nestedFileType;
    if (!nestedFileName.trim() || /[\\/\x00-\x1f]/.test(nestedFileName) || nestedFileName === '.' || nestedFileName === '..') {
      setNestedError('Enter a valid filename.'); return;
    }
    setNestedBusy(true); setNestedError('');
    try {
      const files = await native.createNestedTextFile(selectedNested, nestedPassword, name, nestedFileContent);
      setNestedContents(files); setNestedFileName(''); setNestedFileContent('');
      setNotice('New file encrypted directly inside the nested vault.');
    } catch (reason) { setNestedError(errorMessage(reason)); }
    finally { setNestedBusy(false); setNestedPassword(''); }
  }

  function closeNested() {
    setNestedUnlocked(false); setNestedContents([]); setNestedPassword(''); setNestedFileName(''); setNestedFileContent(''); setNestedError('');
  }

  async function signOut() {
    if (locking.current) return;
    locking.current = true;
    setBusy(true); setError('');
    try {
      await native.lockVault();
      setSession(null); setStoredFiles([]); setDecryptedFiles([]); setNestedContents([]); setNestedUnlocked(false); setNestedFileContent(''); setSelectedNested(null); setLockPassword(''); setNestedPassword(''); setNotice('');
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
      <button type="button" onClick={() => { closeNested(); setPage('overview'); }} aria-current={page === 'overview' ? 'page' : undefined}
        className={"mt-7 flex items-center gap-3 rounded-lg border px-3 py-2.5 text-left text-sm transition-colors " + (page === 'overview' ? 'border-blue-400/10 bg-blue-500/10 text-blue-300' : 'border-transparent text-slate-400 hover:text-blue-300')}>
        <LayoutDashboard size={16} /> Overview
      </button>
      <button type="button" onClick={() => { closeNested(); setPage('vault'); }} aria-current={page === 'vault' ? 'page' : undefined}
        className={"mt-2 flex items-center gap-3 rounded-lg border px-3 py-2.5 text-left text-sm transition-colors " + (page === 'vault' ? 'border-blue-400/10 bg-blue-500/10 text-blue-300' : 'border-transparent text-slate-400 hover:text-blue-300')}>
        <KeyRound size={16} /> My Vault
      </button>
      <button type="button" onClick={() => { closeNested(); setPage('create'); }} aria-current={page === 'create' ? 'page' : undefined}
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
      <section aria-label="Locked Vaults" className="mt-5 border-t border-border pt-4">
        <div className="flex items-center gap-2 px-2 text-[11px] font-medium uppercase tracking-wider text-slate-400"><Lock size={13}/> Locked Vaults <span className="ml-auto font-mono text-[10px]">{nestedVaults.length}</span></div>
        {nestedVaults.length === 0 ? <p className="mt-3 px-2 text-[11px] text-slate-600">No nested vaults yet.</p> : <div className="mt-3 space-y-1">{nestedVaults.map(vault =>
          <button key={vault.id} type="button" onClick={() => { setPage('nested'); closeNested(); setSelectedNested(vault.id); }}
            className={"bv-premium-enter flex w-full items-center gap-2 rounded-lg px-3 py-2 text-left text-xs transition-[background-color,color,transform] duration-300 hover:translate-x-0.5 " + (selectedNested === vault.id ? 'bg-blue-500/10 text-sky-300' : 'text-slate-400 hover:bg-white/[0.04] hover:text-sky-300')}>
            <Lock size={13} className="shrink-0"/><span className="min-w-0 truncate">Vault {vault.id.slice(0, 8)}</span>
          </button>)}</div>}
      </section>
      <div className="mt-auto hidden space-y-3 pt-12 md:block">
        <div className="flex items-center gap-2 text-xs text-slate-400"><span className="size-1.5 rounded-full bg-emerald-400" /> On this Mac</div>
        <p className="font-mono text-[10px] uppercase tracking-widest text-slate-600">Desktop preview · {info?.version ?? '—'}</p>
      </div>
    </aside>

    <main className="mx-auto w-full max-w-6xl px-6 py-8 sm:px-10 sm:py-10">
      <header className="flex flex-wrap items-center justify-between gap-3 border-b border-border pb-6">
        <div className="flex items-center gap-2 text-xs text-slate-400"><HardDrive size={14} /> Personal workspace <span className="mx-1 text-slate-700">/</span><span className="text-slate-200">{page === 'overview' ? 'Overview' : page === 'vault' ? 'My Vault' : page === 'nested' ? 'Nested Vault' : 'Create Files'}</span></div>
        <Button variant="outline" disabled={busy} onClick={() => void signOut()}><LockKeyhole size={14} />{busy ? 'Signing out…' : 'Sign out'}</Button>
      </header>

      <div className={page === 'overview' ? 'bv-page-enter' : 'hidden'} aria-hidden={page !== 'overview'} inert={page !== 'overview'}>
      <section className="pt-6 pb-4">
        <p className="mb-3 font-mono text-[10px] uppercase tracking-[0.2em] text-blue-400">Your private workspace</p>
        <h1 className="text-2xl font-medium tracking-tight sm:text-3xl">Encrypt and decrypt your files.</h1>
        <p className="mt-2 max-w-xl text-xs leading-6 text-slate-400">Open Example Files on the left. Click a sample to select it, or drag it into the workspace below.</p>
      </section>

      <section className="rounded-xl border border-border bg-card px-5 pb-6 pt-1 sm:px-7"><VaultFiles key={nestedVaults.length} vaultId={session.id} chosenExample={chosenExample} chosenDecrypt={chosenDecrypt} onFilesChange={setStoredFiles} onDecryptedChange={setDecryptedFiles} /></section>

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
      {page === 'create' && <div className="bv-page-enter pt-6">
        <p className="mb-3 font-mono text-[10px] uppercase tracking-[0.2em] text-blue-400">In-vault documents</p>
        <h1 className="text-2xl font-medium tracking-tight sm:text-3xl">Create Files</h1>
        <p className="mt-2 max-w-xl text-xs leading-6 text-slate-400">Write a document and encrypt it directly into your unlocked vault. No external Save dialog or plaintext file is created.</p>
        <VaultFileEditor key={nestedVaults.length} vaultId={session.id} onFilesChange={setStoredFiles}/>
      </div>}
      {page === 'nested' && selectedNested && <section key={selectedNested} className="bv-page-enter pt-6">
        <p className="mb-3 font-mono text-[10px] uppercase tracking-[0.2em] text-sky-400">Compartmentalised workspace</p>
        <div className="flex flex-wrap items-center justify-between gap-3">
          <div><h1 className="flex items-center gap-3 text-2xl font-medium tracking-tight sm:text-3xl"><LockKeyhole className="text-sky-300"/> Vault {selectedNested.slice(0, 8)}</h1>
          <p className="mt-2 text-xs text-slate-400">Encrypted within the parent vault · {nestedUnlocked ? 'Authenticated for this view' : 'PIN required'}</p></div>
          <Button variant="outline" onClick={() => { closeNested(); setPage('vault'); }}><Lock size={14}/> {nestedUnlocked ? 'Lock nested vault' : 'Back to parent'}</Button>
        </div>
        {!nestedUnlocked ? <form onSubmit={event => void unlockNested(event)} className="bv-premium-enter mt-7 max-w-md space-y-4 rounded-xl border border-border bg-card p-6">
          <p className="text-sm font-medium">Unlock this compartment</p>
          <p className="text-xs leading-5 text-slate-400">The parent vault must remain unlocked. Enter the separate four-digit PIN to authenticate the nested container.</p>
          <input aria-label="Unlock nested vault PIN" type="password" inputMode="numeric" pattern="[0-9]{4}" maxLength={4} minLength={4} autoComplete="off" placeholder="••••" value={nestedPassword} onChange={event => setNestedPassword(event.target.value.replace(/[^0-9]/g, '').slice(0, 4))} className="w-full rounded-lg border border-border bg-background p-3 text-center font-mono text-xl tracking-[0.6em] outline-none focus:border-sky-400"/>
          <Button type="submit" disabled={nestedBusy || !/^[0-9]{4}$/.test(nestedPassword)}>{nestedBusy ? 'Authenticating…' : 'Unlock compartment'}</Button>
        </form> : <div className="bv-premium-enter mt-7 grid gap-5 lg:grid-cols-[minmax(0,1fr)_minmax(0,1.2fr)]">
          <div className="rounded-xl border border-border bg-card p-5">
            <h2 className="flex items-center gap-2 text-sm font-medium"><FolderOpen size={16} className="text-sky-300"/> Protected files <span className="ml-auto font-mono text-xs text-slate-500">{nestedContents.length}/5</span></h2>
            <p className="mt-2 text-xs text-slate-400">These encrypted envelopes are stored inside this compartment, not alongside parent-vault files.</p>
            <div className="mt-4 space-y-2">{nestedContents.map(file => <div key={file.id} className="bv-premium-enter flex items-center gap-3 rounded-lg border border-border bg-background p-3 text-xs"><Lock size={14} className="shrink-0 text-sky-300"/><span className="min-w-0 flex-1 break-all">{file.name}</span><span className="font-mono text-slate-500">{(file.sizeBytes / 1024).toFixed(1)} KiB</span></div>)}</div>
            {nestedContents.length === 0 && <p className="mt-5 text-xs text-slate-500">No files in this compartment yet.</p>}
            <p className="mt-5 text-[11px] leading-5 text-slate-500">To restore all files to the parent vault, lock this compartment and use the restore control in My Vault. No plaintext export is required.</p>
          </div>
          <form onSubmit={event => void createNestedFile(event)} className="rounded-xl border border-border bg-card p-5">
            <h2 className="flex items-center gap-2 text-sm font-medium"><FilePlus2 size={16} className="text-sky-300"/> Create inside this vault</h2>
            <p className="mt-2 text-xs text-slate-400">The new file is encrypted and added to the nested container. Re-enter your PIN to authorise each write.</p>
            <div className="mt-5 grid gap-3 sm:grid-cols-[1fr_90px]">
              <input aria-label="Nested filename" required maxLength={100} value={nestedFileName} onChange={event => setNestedFileName(event.target.value)} placeholder="Document name" className="min-w-0 rounded-lg border border-border bg-background p-3 text-xs"/>
              <select aria-label="Nested file type" value={nestedFileType} onChange={event => setNestedFileType(event.target.value)} className="rounded-lg border border-border bg-background p-3 text-xs">{['txt','md','json','csv','log'].map(ext => <option key={ext} value={ext}>.{ext}</option>)}</select>
            </div>
            <textarea aria-label="Nested file contents" rows={9} maxLength={5 * 1024 * 1024} value={nestedFileContent} onChange={event => setNestedFileContent(event.target.value)} placeholder="Write your secure document…" className="mt-3 w-full resize-y rounded-lg border border-border bg-[#080f1a] p-3 font-mono text-xs leading-6 outline-none focus:border-sky-400"/>
            <input aria-label="Confirm nested vault PIN" type="password" inputMode="numeric" pattern="[0-9]{4}" minLength={4} maxLength={4} autoComplete="off" placeholder="Confirm 4-digit PIN" value={nestedPassword} onChange={event => setNestedPassword(event.target.value.replace(/[^0-9]/g, '').slice(0, 4))} className="mt-3 w-full rounded-lg border border-border bg-background p-3 text-xs tracking-[0.25em]"/>
            <Button type="submit" className="mt-4" disabled={nestedBusy || nestedContents.length >= 5 || !nestedFileName.trim() || !/^[0-9]{4}$/.test(nestedPassword)}>{nestedBusy ? 'Encrypting…' : 'Encrypt into nested vault'}</Button>
          </form>
        </div>}
        {nestedError && <p role="alert" className="mt-4 rounded-lg border border-red-400/20 bg-red-400/5 p-3 text-xs text-red-300">{nestedError}</p>}
      </section>}
      {page === 'vault' && <div className="bv-page-enter pt-6">
        <p className="mb-3 font-mono text-[10px] uppercase tracking-[0.2em] text-blue-400">Local security</p>
        <h1 className="text-2xl font-medium tracking-tight sm:text-3xl">My Vault</h1>
        <p className="mt-2 max-w-xl text-xs leading-6 text-slate-400">Your encrypted vault, local files, session state and security controls. No cloud connection is required.</p>
        <div className="mt-6 grid gap-4 sm:grid-cols-3">
          <div className="rounded-xl border border-border bg-card p-5"><p className="text-xs text-slate-400">Current vault</p><p className="mt-2 break-all text-sm font-medium">{session.name ?? 'Unlocked vault'}</p></div>
          <div className="rounded-xl border border-border bg-card p-5"><p className="text-xs text-slate-400">Encrypted files</p><p className="mt-2 text-xl font-medium">{storedFiles.length}</p></div>
          <div className="rounded-xl border border-border bg-card p-5"><p className="text-xs text-slate-400">Session security</p><p className="mt-2 text-sm font-medium text-emerald-300">Unlocked · 5-minute auto-lock</p></div>
        </div>
        <section className="bv-premium-enter mt-6 rounded-xl border border-border bg-card p-5 transition-[border-color,box-shadow] duration-300 hover:border-sky-500/20 hover:shadow-[0_12px_40px_#00000018]">
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
        <section className="bv-premium-enter mt-6 rounded-xl border border-border bg-card p-5 transition-[border-color,box-shadow] duration-300 hover:border-sky-500/20 hover:shadow-[0_12px_40px_#00000018]">
          <h2 className="flex items-center gap-2 text-sm font-medium"><Lock size={15} className="text-sky-300"/> Lock selected files into a nested vault</h2>
          <p className="mt-2 text-xs leading-5 text-slate-400">Choose up to five encrypted files. A four-digit PIN adds a convenient second lock alongside the parent vault key. Originals are removed only after the nested record has been written and synchronised. Keep your main vault password and nested PIN safe. A four-digit PIN is a convenience lock, not a substitute for a strong master password. This is cryptographic compartmentalisation within Black Vault, not a separate Qubes VM or OS-level isolation. Locked names stay hidden in the sidebar.</p>
          <form onSubmit={event => void lockFiles(event)} className="mt-4 space-y-3">
            <div className="grid gap-2 sm:grid-cols-2">{storedFiles.map(file =>
              <label key={file.id} className="flex min-w-0 items-center gap-2 rounded-lg border border-border p-3 text-xs"><input type="checkbox" checked={lockIds.includes(file.id)} disabled={nestedBusy || (!lockIds.includes(file.id) && lockIds.length >= 5)}
                onChange={event => setLockIds(previous => event.target.checked ? [...previous, file.id] : previous.filter(id => id !== file.id))}/><span className="min-w-0 break-all">{file.name}</span></label>)}</div>
            {storedFiles.length === 0 && <p className="text-xs text-slate-500">Create or encrypt files first.</p>}
            <div className="grid gap-3 sm:grid-cols-2">
              <input aria-label="Nested vault name" placeholder="Nested vault name" value={lockName} onChange={event => setLockName(event.target.value)} required maxLength={80} disabled={nestedBusy} className="rounded-lg border border-border bg-background px-3 py-2.5 text-xs"/>
              <input aria-label="Nested vault PIN" type="password" inputMode="numeric" pattern="[0-9]{4}" title="Enter exactly four digits" placeholder="4-digit PIN" autoComplete="new-password" minLength={4} maxLength={4} value={lockPassword} onChange={event => setLockPassword(event.target.value.replace(/[^0-9]/g, '').slice(0, 4))} required disabled={nestedBusy} className="rounded-lg border border-border bg-background px-3 py-2.5 text-xs tracking-[0.3em] transition-[border-color,box-shadow] duration-300 focus:border-sky-400 focus:shadow-[0_0_0_3px_#38bdf815]"/>
            </div>
            <Button type="submit" disabled={nestedBusy || lockIds.length === 0 || !lockName.trim() || !/^[0-9]{4}$/.test(lockPassword)}><Lock size={14}/>{nestedBusy ? 'Securing files…' : 'Lock selected files'}</Button>
          </form>
        </section>
        {selectedNested && page === 'vault' && <section className="bv-premium-enter mt-6 rounded-xl border border-border bg-card p-5 transition-[border-color,box-shadow] duration-300 hover:border-sky-500/20 hover:shadow-[0_12px_40px_#00000018]">
          <h2 className="flex items-center gap-2 text-sm font-medium"><LockOpen size={15} className="text-sky-300"/> Nested vault {selectedNested.slice(0, 8)}</h2>
          <p className="mt-2 text-xs text-slate-400">Enter your four-digit PIN to inspect the protected file list, or restore its encrypted files to the parent vault. Restoration removes the nested container only after all encrypted files are saved.</p>
          <form onSubmit={event => void unlockNested(event)} className="mt-4 flex flex-wrap gap-3">
            <input aria-label="Unlock nested vault PIN" type="password" inputMode="numeric" pattern="[0-9]{4}" title="Enter your four-digit PIN" minLength={4} maxLength={4} value={nestedPassword} onChange={event => setNestedPassword(event.target.value.replace(/[^0-9]/g, '').slice(0, 4))} required disabled={nestedBusy} autoComplete="off" placeholder="4-digit PIN" className="min-w-0 flex-1 rounded-lg border border-border bg-background px-3 py-2.5 text-xs tracking-[0.3em] transition-[border-color,box-shadow] duration-300 focus:border-sky-400 focus:shadow-[0_0_0_3px_#38bdf815]"/>
            <Button type="submit" disabled={nestedBusy || !/^[0-9]{4}$/.test(nestedPassword)}><LockOpen size={14}/>{nestedBusy ? 'Unlocking…' : 'Inspect vault'}</Button>
            <Button type="button" variant="outline" disabled={nestedBusy || !/^[0-9]{4}$/.test(nestedPassword)} onClick={() => void restoreNested()}>Restore to parent</Button>
            <Button type="button" variant="outline" onClick={() => { setSelectedNested(null); setNestedContents([]); setNestedPassword(''); }}>Close</Button>
          </form>
          {nestedContents.length > 0 && <div className="bv-premium-enter mt-4 space-y-2">{nestedContents.map(file => <p key={file.id} className="rounded-lg border border-border p-3 text-xs">{file.name}</p>)}</div>}
        </section>}
        {nestedError && <p role="alert" className="mt-3 text-xs text-red-300">{nestedError}</p>}
        <VaultWorkspace ready={Boolean(info)} onLocked={() => { setStoredFiles([]); setDecryptedFiles([]); setSession(null); }} />
      </div>}

      {error && <p role="alert" className="mt-4 rounded-lg border border-red-400/20 bg-red-400/5 px-4 py-3 text-sm text-red-300">{error}</p>}
      <p role="status" className="mt-4 flex items-center gap-2 text-xs text-slate-400">{notice && <><Check size={14} className="text-emerald-400" />{notice}</>}</p>
      <footer className="mt-8 flex flex-wrap justify-between gap-2 border-t border-border pt-5 font-mono text-[10px] uppercase tracking-widest text-slate-600"><span>Black Vault · Desktop foundation</span><span>{info ? `${info.platform} / ${info.architecture}` : 'Native connection pending'} · Auto-lock after {DEFAULT_IDLE_MS / 60000} min idle</span></footer>
    </main>
  </div>;
}
