import { useEffect, useState } from 'react';
import { ArrowUpRight, Check, File, FolderOpen, HardDrive, LayoutDashboard, LockKeyhole, ShieldCheck, X } from 'lucide-react';
import { Button } from '@/components/ui/button';
import VaultWorkspace from '@/components/VaultWorkspace';
import AuthGate from '@/components/AuthGate';
import SplashScreen from '@/components/SplashScreen';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { errorMessage, formatFileSize, native, type DesktopInfo, type SelectedFile, type VaultSummary } from '@/lib/native';

export default function App() {
  const [info, setInfo] = useState<DesktopInfo | null>(null);
  const [session, setSession] = useState<VaultSummary | null>(null);
  const [booting, setBooting] = useState(true);
  const [file, setFile] = useState<SelectedFile | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const [notice, setNotice] = useState('');

  useEffect(() => {
    let active = true;
    // Fail closed: a webview reload must not inherit an unlocked Rust vault session.
    Promise.all([native.lockVault(), native.getDesktopInfo(), new Promise<void>(resolve => setTimeout(resolve, 1100))]).then(([, value]) => {
      if (active) setInfo(value);
    }).catch(reason => { if (active) setError(errorMessage(reason)); })
      .finally(() => { if (active) setBooting(false); });
    return () => { active = false; };
  }, []);

  async function signOut() {
    setBusy(true); setError('');
    try {
      await native.lockVault();
      setSession(null); setFile(null); setNotice('');
    } catch (reason) { setError(errorMessage(reason)); }
    finally { setBusy(false); }
  }

  async function selectFile() {
    setBusy(true); setError(''); setNotice('');
    try {
      const selected = await native.selectFile();
      if (selected) { setFile(selected); setNotice('File selected. Nothing has been imported.'); }
      else setNotice('Selection cancelled.');
    } catch (reason) { setError(errorMessage(reason)); }
    finally { setBusy(false); }
  }

  if (booting) return <SplashScreen />;
  if (!info) return <main className="grid place-items-center p-10"><p role="alert" className="text-sm text-red-300">{error || 'Native desktop unavailable.'}</p></main>;
  if (!session) return <AuthGate onAuthenticated={setSession} />;

  return <div className="grid grid-cols-1 md:grid-cols-[204px_1fr]">
    <aside className="flex flex-col border-b border-border bg-[#0b1018] p-5 md:border-r md:border-b-0">
      <div className="flex items-center gap-3 py-2">
        <div className="flex size-9 items-center justify-center rounded-xl border border-blue-400/25 bg-blue-500/10"><LockKeyhole size={18} className="text-blue-400" /></div>
        <span className="text-xs font-semibold tracking-[0.12em]">BLACK VAULT</span>
      </div>
      <div className="mt-7 flex items-center gap-3 rounded-lg border border-blue-400/10 bg-blue-500/10 px-3 py-2.5 text-sm text-blue-300" aria-current="page"><LayoutDashboard size={16} /> Overview</div>
      <div className="mt-auto hidden space-y-3 pt-12 md:block">
        <div className="flex items-center gap-2 text-xs text-slate-400"><span className="size-1.5 rounded-full bg-emerald-400" /> On this Mac</div>
        <p className="font-mono text-[10px] uppercase tracking-widest text-slate-600">Desktop preview · {info?.version ?? '—'}</p>
      </div>
    </aside>

    <main className="mx-auto w-full max-w-6xl px-6 py-8 sm:px-10 sm:py-10">
      <header className="flex flex-wrap items-center justify-between gap-3 border-b border-border pb-6">
        <div className="flex items-center gap-2 text-xs text-slate-400"><HardDrive size={14} /> Personal workspace <span className="mx-1 text-slate-700">/</span><span className="text-slate-200">Overview</span></div>
        <Button variant="outline" disabled={busy} onClick={() => void signOut()}><LockKeyhole size={14} />{busy ? 'Signing out…' : 'Sign out'}</Button>
      </header>

      <section className="py-9 sm:py-11">
        <p className="mb-3 font-mono text-[10px] uppercase tracking-[0.2em] text-blue-400">Your private workspace</p>
        <h1 className="text-3xl font-medium tracking-tight sm:text-4xl">Your vault, on your Mac.</h1>
        <p className="mt-4 max-w-xl text-sm leading-6 text-slate-400">A dedicated home for your private files. Start by selecting a file from your Mac.</p>
      </section>

      <div className="grid gap-4 lg:grid-cols-3">
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
          <CardContent><p className="text-lg">{file ? '1 file' : 'No file selected'}</p><p className="mt-1 text-xs text-slate-500">{file ? formatFileSize(file.sizeBytes) : 'Choose a file to view its details'}</p></CardContent>
        </Card>
      </div>

      <VaultWorkspace ready={Boolean(info)} onLocked={() => { setSession(null); setFile(null); }} />

      <section className="mt-8 overflow-hidden rounded-xl border border-border bg-card" aria-labelledby="files-title">
        <div className="flex flex-wrap items-center justify-between gap-4 border-b border-border px-6 py-5">
          <div><h2 id="files-title" className="text-sm font-medium">File workspace</h2><p className="mt-1 text-xs text-slate-500">Select a file using the macOS file picker.</p></div>
          <Button onClick={() => void selectFile()} disabled={busy || !info}><FolderOpen size={15} />{busy ? 'Selecting…' : 'Select a file'}<ArrowUpRight size={14} /></Button>
        </div>
        {file ? <div className="flex flex-wrap items-center gap-4 px-6 py-9">
          <div className="rounded-xl border border-border bg-muted p-4"><File size={24} className="text-blue-300" /></div>
          <div className="min-w-0 flex-1"><h3 className="break-all text-sm font-medium">{file.name}</h3><p className="mt-2 font-mono text-xs text-slate-500">{formatFileSize(file.sizeBytes)} · Selected only</p></div>
          <Button variant="outline" disabled={busy} onClick={() => { setFile(null); setNotice('Selection cleared.'); }}><X size={14} />Clear selection</Button>
        </div> : <div className="grid justify-items-center px-6 py-12 text-center">
          <div className="mb-5 rounded-2xl border border-border bg-muted/50 p-4"><FolderOpen size={28} strokeWidth={1.3} className="text-slate-500" /></div>
          <h3 className="text-sm text-slate-200">A clean start.</h3>
          <p className="mt-2 max-w-sm text-xs leading-6 text-slate-500">Choose a file to see its name and size here.<br />Your file stays in its original location.</p>
        </div>}
        <div className="flex items-start gap-2 border-t border-border bg-[#0b111b] px-6 py-3.5 text-xs leading-5 text-slate-400"><ShieldCheck size={14} className="mt-0.5 shrink-0 text-blue-400" />This preview checks file details only. It does not read, copy or encrypt file contents.</div>
      </section>
      {error && <p role="alert" className="mt-4 rounded-lg border border-red-400/20 bg-red-400/5 px-4 py-3 text-sm text-red-300">{error}</p>}
      <p role="status" className="mt-4 flex items-center gap-2 text-xs text-slate-400">{notice && <><Check size={14} className="text-emerald-400" />{notice}</>}</p>
      <footer className="mt-8 flex flex-wrap justify-between gap-2 border-t border-border pt-5 font-mono text-[10px] uppercase tracking-widest text-slate-600"><span>Black Vault · Desktop foundation</span><span>{info ? `${info.platform} / ${info.architecture}` : 'Native connection pending'}</span></footer>
    </main>
  </div>;
}
