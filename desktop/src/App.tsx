import { lazy, Suspense, useEffect, useRef, useState } from 'react';
import { AnimatePresence, motion, useReducedMotion, type Variants } from 'framer-motion';
import { Check, ChevronDown, ContactRound, File, FileLock2, FolderOpen, HardDrive, LayoutDashboard, LockKeyhole, FilePlus2, Lock, LockOpen } from 'lucide-react';
import { Button } from '@/components/ui/button';
import VaultWorkspace from '@/components/VaultWorkspace';
import VaultFiles from '@/components/VaultFiles';
import VaultFileEditor from '@/components/VaultFileEditor';
import ExampleFiles from '@/components/ExampleFiles';
import VaultAccess from '@/components/VaultAccess';
import SplashScreen from '@/components/SplashScreen';
import VaultMark from '@/components/VaultMark';
import DashboardOverview from '@/components/DashboardOverview';
import { DEFAULT_IDLE_MS, hasSessionExpired } from '@/lib/sessionTimeout';
import { errorMessage, native, type DesktopInfo, type VaultSummary, type VaultFile, type NestedVaultSummary } from '@/lib/native';

const TrustedContacts = lazy(() => import('@/components/TrustedContacts'));

const pageStagger: Variants = {
  hidden: {},
  show: { transition: { staggerChildren: 0.075, delayChildren: 0.035 } },
};

const pageItem: Variants = {
  hidden: { opacity: 0, y: 14, scale: 0.996 },
  show: (delay = 0) => ({
    opacity: 1,
    y: 0,
    scale: 1,
    transition: { duration: 0.34, delay, ease: [0.22, 1, 0.36, 1] },
  }),
};

export default function App() {
  const [info, setInfo] = useState<DesktopInfo | null>(null);
  const [session, setSession] = useState<VaultSummary | null>(null);
  const reduceMotion = useReducedMotion();
  const motionDisabled = Boolean(reduceMotion || typeof window === 'undefined' || !window.matchMedia);

  function enterDashboard(vault: VaultSummary, destination: 'encrypt' | 'decrypt' = 'decrypt') {
    // Native authentication is the only transition that creates an open vault session.
    setSession(vault);
    setPage(destination);
  }
  const [booting, setBooting] = useState(true);
  const [examplesOpen, setExamplesOpen] = useState(false);
  const [page, setPage] = useState<'overview' | 'encrypt' | 'decrypt' | 'nested' | 'contacts'>('overview');

  function navigateTo(nextPage: 'overview' | 'encrypt' | 'decrypt' | 'nested' | 'contacts') {
    if (nextPage === page) return;
    setPage(nextPage);
  }
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
  const [selectedProtectedFile, setSelectedProtectedFile] = useState<string | null>(null);
  const [nestedEditing, setNestedEditing] = useState(false);
  const [nestedDeleteConfirm, setNestedDeleteConfirm] = useState(false);
  const [parentTransferId, setParentTransferId] = useState('');
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
    Promise.all([native.lockVault(), native.getDesktopInfo()]).then(([, value]) => {
      if (active) setInfo(value);
    }).catch(reason => { if (active) setError(errorMessage(reason)); });

    const reducedMotion = Boolean(window.matchMedia?.('(prefers-reduced-motion: reduce)').matches);
    const launchTimer = window.setTimeout(() => {
      if (active) setBooting(false);
    }, reducedMotion ? 250 : 3100);

    return () => {
      active = false;
      window.clearTimeout(launchTimer);
    };
  }, []);

  useEffect(() => {
    if (!session) { setNestedVaults([]); setSelectedNested(null); setNestedContents([]); setNestedPassword(''); setNestedUnlocked(false); setNestedFileContent(''); return; }
    let active = true;
    Promise.all([native.listNestedVaults(), native.listVaultFiles()])
      .then(([nested, files]) => {
        if (!active) return;
        setNestedVaults(nested);
        setStoredFiles(files);
      })
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
      setNestedContents(files); setNestedFileName(''); setNestedFileContent(''); setSelectedProtectedFile(null); setNestedEditing(false);
      setNotice('New file encrypted directly inside the nested vault.');
    } catch (reason) { setNestedError(errorMessage(reason)); }
    finally { setNestedBusy(false); setNestedPassword(''); }
  }

  async function moveParentToNested(event: React.FormEvent) {
    event.preventDefault();
    if (!selectedNested || !nestedUnlocked || !parentTransferId || nestedBusy || !/^[0-9]{4}$/.test(nestedPassword)) return;
    setNestedBusy(true); setNestedError('');
    try {
      const updated = await native.transferParentFileToNested(selectedNested, nestedPassword, parentTransferId);
      setNestedContents(updated); setStoredFiles(await native.listVaultFiles());
      setParentTransferId(''); setLockIds(previous => previous.filter(id => id !== parentTransferId));
      setNotice('Encrypted file transferred into the nested vault. No plaintext was exported.');
    } catch (reason) { setNestedError(errorMessage(reason)); }
    finally { setNestedPassword(''); setNestedBusy(false); }
  }

  const selectedProtected = nestedContents.find(file => file.id === selectedProtectedFile) ?? null;

  function selectProtected(file: VaultFile) {
    setSelectedProtectedFile(file.id); setNestedEditing(false); setNestedDeleteConfirm(false);
    setNestedFileName(file.name.replace(/\.[^.]+$/, ''));
    setNestedFileType(file.name.split('.').pop()?.toLowerCase() ?? 'txt');
    setNestedFileContent(''); setNestedPassword(''); setNestedError('');
  }

  async function readProtected() {
    if (!selectedNested || !selectedProtected || !/^[0-9]{4}$/.test(nestedPassword) || nestedBusy) return;
    setNestedBusy(true); setNestedError('');
    try {
      const content = await native.readNestedTextFile(selectedNested, nestedPassword, selectedProtected.id);
      setNestedFileContent(content); setNestedEditing(true);
      setNotice('Authenticated document opened in the session editor.');
    } catch (reason) { setNestedError(errorMessage(reason)); }
    finally { setNestedPassword(''); setNestedBusy(false); }
  }

  async function manageProtected(action: 'edit' | 'delete' | 'transfer_to_parent') {
    if (!selectedNested || !selectedProtected || !/^[0-9]{4}$/.test(nestedPassword) || nestedBusy) return;
    if (action === 'delete' && !nestedDeleteConfirm) { setNestedDeleteConfirm(true); return; }
    setNestedBusy(true); setNestedError('');
    try {
      const updated = await native.manageNestedFile(selectedNested, nestedPassword, selectedProtected.id, action,
        action === 'edit' ? nestedFileName.trim() + '.' + nestedFileType : undefined,
        action === 'edit' ? nestedFileContent : undefined);
      setNestedContents(updated);
      if (action === 'transfer_to_parent') setStoredFiles(await native.listVaultFiles());
      setSelectedProtectedFile(null); setNestedEditing(false); setNestedDeleteConfirm(false); setNestedFileContent(''); setNestedFileName('');
      setNotice(action === 'edit' ? 'Protected document re-encrypted and updated.' : action === 'delete' ? 'Protected file permanently deleted.' : 'Encrypted file transferred to the parent vault.');
    } catch (reason) { setNestedError(errorMessage(reason)); }
    finally { setNestedPassword(''); setNestedBusy(false); }
  }

  function closeNested() {
    setNestedUnlocked(false); setNestedContents([]); setNestedPassword(''); setNestedFileName(''); setNestedFileContent(''); setNestedError(''); setSelectedProtectedFile(null); setNestedEditing(false); setNestedDeleteConfirm(false); setParentTransferId('');
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

  if (!info) {
    if (booting) return <SplashScreen />;
    return <main className="grid place-items-center p-10"><p role="alert" className="text-sm text-red-300">{error || 'Native desktop unavailable.'}</p></main>;
  }

  return <>
    {booting && <SplashScreen />}
    <motion.div
      className="bv-dashboard-shell grid grid-cols-1 md:grid-cols-[260px_minmax(0,1fr)]"
      initial={false}
      animate={motionDisabled || !booting
        ? { opacity: 1, y: 0, scale: 1 }
        : { opacity: 0, y: 10, scale: 0.995 }}
      transition={{ duration: motionDisabled ? 0 : 0.45, ease: [0.22, 1, 0.36, 1] }}
    >
    <aside className="flex flex-col border-b border-border bg-[#0b1018] p-5 md:border-r md:border-b-0">
      <div className="flex items-center gap-3 py-2">
        <div className="flex size-10 items-center justify-center rounded-xl border border-blue-400/25 bg-blue-500/10"><VaultMark /></div>
        <span className="text-xs font-semibold tracking-[0.12em]">BLACK VAULT</span>
      </div>
      <button type="button" onClick={() => { closeNested(); navigateTo('overview'); }} aria-current={page === 'overview' ? 'page' : undefined}
        className={"mt-7 flex items-center gap-3 rounded-lg border px-3 py-2.5 text-left text-sm transition-colors " + (page === 'overview' ? 'border-blue-400/10 bg-blue-500/10 text-blue-300' : 'border-transparent text-slate-400 hover:text-blue-300')}>
        <LayoutDashboard size={16} /> Overview
      </button>
      <button type="button" onClick={() => { closeNested(); navigateTo('encrypt'); }} aria-current={page === 'encrypt' ? 'page' : undefined}
        className={"mt-2 flex items-center gap-3 rounded-lg border px-3 py-2.5 text-left text-sm transition-colors " + (page === 'encrypt' ? 'border-blue-400/10 bg-blue-500/10 text-blue-300' : 'border-transparent text-slate-400 hover:text-blue-300')}>
        <FileLock2 size={16} /> Encryption
      </button>
      <button type="button" onClick={() => { closeNested(); navigateTo('decrypt'); }} aria-current={page === 'decrypt' ? 'page' : undefined}
        className={"mt-2 flex items-center gap-3 rounded-lg border px-3 py-2.5 text-left text-sm transition-colors " + (page === 'decrypt' ? 'border-blue-400/10 bg-blue-500/10 text-blue-300' : 'border-transparent text-slate-400 hover:text-blue-300')}>
        <LockOpen size={16} /> Decryption
      </button>
      <button type="button" onClick={() => { closeNested(); navigateTo('contacts'); }} aria-current={page === 'contacts' ? 'page' : undefined}
        className={"mt-2 flex items-center gap-3 rounded-lg border px-3 py-2.5 text-left text-sm transition-colors " + (page === 'contacts' ? 'border-blue-400/10 bg-blue-500/10 text-blue-300' : 'border-transparent text-slate-400 hover:text-blue-300')}>
        <ContactRound size={16} /> Trusted Contacts
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
            onSelect={id => { navigateTo('encrypt'); setChosenExample(current => ({ id, nonce: (current?.nonce ?? 0) + 1 })); }}
            onDecryptSelect={id => { navigateTo('decrypt'); setChosenDecrypt(current => ({ id, nonce: (current?.nonce ?? 0) + 1 })); }} />
        </div>
      </div>
      <section aria-label="Locked Vaults" className="mt-5 border-t border-border pt-4">
        <div className="flex items-center gap-2 px-2 text-[11px] font-medium uppercase tracking-wider text-slate-400"><Lock size={13}/> Locked Vaults <span className="ml-auto font-mono text-[10px]">{nestedVaults.length}</span></div>
        {nestedVaults.length === 0 ? <p className="mt-3 px-2 text-[11px] text-slate-600">No nested vaults yet.</p> : <div className="mt-3 space-y-1">{nestedVaults.map(vault =>
          <button key={vault.id} type="button" onClick={() => { navigateTo('nested'); closeNested(); setSelectedNested(vault.id); }}
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
      <motion.header
        className="flex flex-wrap items-center justify-between gap-3 border-b border-border pb-6"
        initial={motionDisabled ? false : { opacity: 0, y: -8 }}
        animate={booting ? { opacity: 0, y: -8 } : { opacity: 1, y: 0 }}
        transition={{ duration: motionDisabled ? 0 : 0.32, delay: motionDisabled ? 0 : 0.04, ease: [0.22, 1, 0.36, 1] }}
      >
        <div className="flex items-center gap-2 text-xs text-slate-400"><HardDrive size={14} /> Personal workspace <span className="mx-1 text-slate-700">/</span><span className="text-slate-200">{page === 'overview' ? 'Overview' : page === 'encrypt' ? 'Encryption' : page === 'decrypt' ? 'Decryption' : page === 'nested' ? 'Nested Vault' : 'Trusted Contacts'}</span></div>
        {session && <Button variant="outline" disabled={busy} onClick={() => void signOut()}><LockKeyhole size={14} />{busy ? 'Locking…' : 'Lock vault'}</Button>}
      </motion.header>

      <AnimatePresence mode={motionDisabled ? "sync" : "wait"} initial={false}>
        <motion.div
          key={`${page}-${booting ? 'booting' : 'ready'}`}
          initial={false}
          animate={{ opacity: 1 }}
          exit={motionDisabled ? undefined : { opacity: 0, y: -6 }}
          transition={{ duration: motionDisabled ? 0 : 0.16, ease: [0.4, 0, 1, 1] }}
        >
      {page === 'overview' && <DashboardOverview currentVault={session} files={storedFiles} onCreateVault={intent => navigateTo(intent)} />}

      {page === 'encrypt' && <motion.div
        className="pt-6"
        variants={pageStagger}
        initial={motionDisabled ? false : 'hidden'}
        animate="show"
      >
        <motion.div variants={pageItem}>
          <p className="mb-3 font-mono text-[10px] uppercase tracking-[0.2em] text-blue-400">Protect local files</p>
          <h1 className="text-2xl font-medium tracking-tight sm:text-3xl">Encryption</h1>
          <p className="mt-2 max-w-xl text-xs leading-6 text-slate-400">Choose or drop one file. Black Vault scans it with the local policy gate, encrypts it natively and stores only the encrypted vault copy.</p>
        </motion.div>
        <motion.div variants={pageItem}>
          {session
            ? <section className="mt-6 rounded-xl border border-border bg-card p-5 sm:p-7"><VaultFiles operation="encrypt" key={'encrypt-' + nestedVaults.length} vaultId={session.id} chosenExample={chosenExample} onFilesChange={setStoredFiles} /></section>
            : <VaultAccess onAuthenticated={vault => enterDashboard(vault, 'encrypt')} />}
        </motion.div>
      </motion.div>}

      {page === 'decrypt' && <motion.div
        className="pt-6"
        variants={pageStagger}
        initial={motionDisabled ? false : 'hidden'}
        animate="show"
      >
        <motion.div variants={pageItem}>
          <p className="mb-3 font-mono text-[10px] uppercase tracking-[0.2em] text-blue-400">Verify protected files</p>
          <h1 className="text-2xl font-medium tracking-tight sm:text-3xl">Decryption</h1>
          <p className="mt-2 max-w-xl text-xs leading-6 text-slate-400">Select an encrypted vault file. Authentication and decryption happen in native memory; the encrypted original remains in the vault.</p>
        </motion.div>
        <motion.div variants={pageItem}>
          {session
            ? <section className="mt-6 rounded-xl border border-border bg-card p-5 sm:p-7"><VaultFiles operation="decrypt" key={'decrypt-' + nestedVaults.length} vaultId={session.id} chosenDecrypt={chosenDecrypt} onFilesChange={setStoredFiles} onDecryptedChange={setDecryptedFiles} /></section>
            : <VaultAccess onAuthenticated={enterDashboard} />}
        </motion.div>
      </motion.div>}
      {page === 'contacts' && <Suspense fallback={<div className="pt-8 text-xs text-slate-500">Loading trusted contacts…</div>}><TrustedContacts /></Suspense>}
      {page === 'decrypt' && session && <motion.section
        className="mt-6 border-t border-border pt-6"
        variants={pageItem}
        custom={0.16}
        initial={motionDisabled ? false : 'hidden'}
        animate="show"
      >
        <p className="mb-2 font-mono text-[10px] uppercase tracking-[0.2em] text-sky-400">Authenticated vault workspace</p>
        <h2 className="text-lg font-medium">Open, examine and edit protected documents</h2>
        <p className="mt-2 max-w-2xl text-xs leading-6 text-slate-400">Supported text documents are decrypted only into the active session editor. Saving re-encrypts the document into the vault; no plaintext working copy is written to disk.</p>
        <VaultFileEditor key={nestedVaults.length} vaultId={session.id} onFilesChange={setStoredFiles}/>
      </motion.section>}
      {page === 'nested' && session && selectedNested && <section key={selectedNested} className="bv-page-enter pt-6">
        <p className="mb-3 font-mono text-[10px] uppercase tracking-[0.2em] text-sky-400">Compartmentalised workspace</p>
        <div className="flex flex-wrap items-center justify-between gap-3">
          <div><h1 className="flex items-center gap-3 text-2xl font-medium tracking-tight sm:text-3xl"><LockKeyhole className="text-sky-300"/> Vault {selectedNested.slice(0, 8)}</h1>
          <p className="mt-2 text-xs text-slate-400">Encrypted within the parent vault · {nestedUnlocked ? 'Authenticated for this view' : 'PIN required'}</p></div>
          <Button variant="outline" onClick={() => { closeNested(); navigateTo('decrypt'); }}><Lock size={14}/> {nestedUnlocked ? 'Lock nested vault' : 'Back to parent'}</Button>
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
            <div className="mt-4 space-y-2">{nestedContents.map(file => <button type="button" key={file.id} onClick={() => selectProtected(file)} aria-pressed={selectedProtectedFile === file.id} className={"bv-premium-enter flex w-full items-center gap-3 rounded-lg border p-3 text-left text-xs transition-colors " + (selectedProtectedFile === file.id ? 'border-sky-400/50 bg-sky-400/10 text-sky-100' : 'border-border bg-background hover:border-sky-400/30')}><Lock size={14} className="shrink-0 text-sky-300"/><span className="min-w-0 flex-1 break-all">{file.name}</span><span className="shrink-0 font-mono text-slate-500">{(file.sizeBytes / 1024).toFixed(1)} KiB</span></button>)}</div>
            {nestedContents.length === 0 && <p className="mt-5 text-xs text-slate-500">No files in this compartment yet.</p>}
            <form onSubmit={event => void moveParentToNested(event)} className="mt-6 space-y-3 border-t border-border pt-5">
              <h3 className="text-xs font-medium text-sky-300">Move from parent vault</h3>
              <p className="text-[11px] leading-5 text-slate-500">Select one encrypted parent file. The original is removed only after its protected copy is synchronised here.</p>
              <select aria-label="Parent file to transfer" value={parentTransferId} onChange={event => setParentTransferId(event.target.value)} disabled={nestedBusy || nestedContents.length >= 5} className="w-full min-w-0 rounded-lg border border-border bg-background p-3 text-xs">
                <option value="">Select a parent file…</option>
                {storedFiles.filter(file => !nestedContents.some(child => child.id === file.id)).map(file => <option key={file.id} value={file.id}>{file.name}</option>)}
              </select>
              <input aria-label="PIN to move parent file" type="password" inputMode="numeric" pattern="[0-9]{4}" minLength={4} maxLength={4} autoComplete="off" placeholder="Confirm 4-digit PIN" value={nestedPassword} onChange={event => setNestedPassword(event.target.value.replace(/[^0-9]/g, '').slice(0, 4))} className="w-full rounded-lg border border-border bg-background p-3 text-xs tracking-[0.25em]"/>
              <Button type="submit" variant="outline" disabled={nestedBusy || nestedContents.length >= 5 || !parentTransferId || !/^[0-9]{4}$/.test(nestedPassword)}>Move encrypted file into vault</Button>
            </form>
            <p className="mt-5 text-[11px] leading-5 text-slate-500">To restore all files to the parent vault, lock this compartment and use the restore control in My Vault. No plaintext export is required.</p>
          </div>
          <div className="rounded-xl border border-border bg-card p-5">
            {selectedProtected ? <div className="space-y-4">
              <h2 className="flex items-center gap-2 text-sm font-medium"><File size={16} className="text-sky-300"/> Protected file details</h2>
              <dl className="grid gap-3 rounded-lg border border-border bg-background p-4 text-xs">
                <div><dt className="text-slate-500">Filename</dt><dd className="mt-1 break-all text-slate-100">{selectedProtected.name}</dd></div>
                <div><dt className="text-slate-500">Size</dt><dd className="mt-1 font-mono">{selectedProtected.sizeBytes.toLocaleString()} bytes</dd></div>
                <div><dt className="text-slate-500">File ID</dt><dd className="mt-1 break-all font-mono text-[11px]">{selectedProtected.id}</dd></div>
                <div><dt className="text-slate-500">Protection</dt><dd className="mt-1 text-sky-300">Parent encryption + nested PIN layer</dd></div>
              </dl>
              {nestedEditing && <div className="grid gap-3">
                <div className="grid gap-2 sm:grid-cols-[1fr_90px]"><input aria-label="Edit protected filename" value={nestedFileName} maxLength={100} onChange={event => setNestedFileName(event.target.value)} className="min-w-0 rounded-lg border border-border bg-background p-3 text-xs"/><select aria-label="Edit protected file type" value={nestedFileType} onChange={event => setNestedFileType(event.target.value)} className="rounded-lg border border-border bg-background p-3 text-xs">{['txt','md','json','csv','log'].map(ext => <option key={ext} value={ext}>.{ext}</option>)}</select></div>
                <textarea aria-label="Edit protected file contents" value={nestedFileContent} onChange={event => setNestedFileContent(event.target.value)} rows={9} spellCheck={false} className="w-full resize-y rounded-lg border border-border bg-[#080f1a] p-3 font-mono text-xs leading-6"/>
              </div>}
              <p className="text-[11px] leading-5 text-slate-500">Re-enter the four-digit PIN to authorise each read or change. File content is never exported to disk.</p>
              <input aria-label="Protected file operation PIN" type="password" inputMode="numeric" pattern="[0-9]{4}" minLength={4} maxLength={4} autoComplete="off" placeholder="Confirm 4-digit PIN" value={nestedPassword} onChange={event => setNestedPassword(event.target.value.replace(/[^0-9]/g, '').slice(0, 4))} className="w-full rounded-lg border border-border bg-background p-3 text-xs tracking-[0.25em]"/>
              <div className="flex flex-wrap gap-2">
                {!nestedEditing && <Button type="button" variant="outline" disabled={nestedBusy || !/^[0-9]{4}$/.test(nestedPassword)} onClick={() => void readProtected()}>Open text editor</Button>}
                {nestedEditing && <Button type="button" disabled={nestedBusy || !nestedFileName.trim() || !/^[0-9]{4}$/.test(nestedPassword)} onClick={() => void manageProtected('edit')}>Save encrypted changes</Button>}
                <Button type="button" variant="outline" disabled={nestedBusy || !/^[0-9]{4}$/.test(nestedPassword)} onClick={() => void manageProtected('transfer_to_parent')}>Move to parent</Button>
                <Button type="button" variant="outline" disabled={nestedBusy || !/^[0-9]{4}$/.test(nestedPassword)} onClick={() => void manageProtected('delete')}>{nestedDeleteConfirm ? 'Confirm permanent delete' : 'Delete file'}</Button>
                <Button type="button" variant="outline" onClick={() => { setSelectedProtectedFile(null); setNestedEditing(false); setNestedFileContent(''); setNestedPassword(''); setNestedDeleteConfirm(false); }}>Close details</Button>
              </div>
            </div> : <form onSubmit={event => void createNestedFile(event)}>
              <h2 className="flex items-center gap-2 text-sm font-medium"><FilePlus2 size={16} className="text-sky-300"/> Create inside this vault</h2>
              <p className="mt-2 text-xs text-slate-400">Select a protected file on the left to view its information and actions here, or create a new encrypted document.</p>
              <div className="mt-5 grid gap-3 sm:grid-cols-[1fr_90px]">
                <input aria-label="Nested filename" required maxLength={100} value={nestedFileName} onChange={event => setNestedFileName(event.target.value)} placeholder="Document name" className="min-w-0 rounded-lg border border-border bg-background p-3 text-xs"/>
                <select aria-label="Nested file type" value={nestedFileType} onChange={event => setNestedFileType(event.target.value)} className="rounded-lg border border-border bg-background p-3 text-xs">{['txt','md','json','csv','log'].map(ext => <option key={ext} value={ext}>.{ext}</option>)}</select>
              </div>
              <textarea aria-label="Nested file contents" rows={9} maxLength={5 * 1024 * 1024} value={nestedFileContent} onChange={event => setNestedFileContent(event.target.value)} placeholder="Write your secure document…" className="mt-3 w-full resize-y rounded-lg border border-border bg-[#080f1a] p-3 font-mono text-xs leading-6 outline-none focus:border-sky-400"/>
              <input aria-label="Confirm nested vault PIN" type="password" inputMode="numeric" pattern="[0-9]{4}" minLength={4} maxLength={4} autoComplete="off" placeholder="Confirm 4-digit PIN" value={nestedPassword} onChange={event => setNestedPassword(event.target.value.replace(/[^0-9]/g, '').slice(0, 4))} className="mt-3 w-full rounded-lg border border-border bg-background p-3 text-xs tracking-[0.25em]"/>
              <Button type="submit" className="mt-4" disabled={nestedBusy || nestedContents.length >= 5 || !nestedFileName.trim() || !/^[0-9]{4}$/.test(nestedPassword)}>{nestedBusy ? 'Encrypting…' : 'Encrypt into nested vault'}</Button>
            </form>}
          </div>
        </div>}
        {nestedError && <p role="alert" className="mt-4 rounded-lg border border-red-400/20 bg-red-400/5 p-3 text-xs text-red-300">{nestedError}</p>}
      </section>}
      {page === 'decrypt' && session && <motion.div
        className="mt-6 border-t border-border pt-6"
        variants={pageItem}
        custom={0.24}
        initial={motionDisabled ? false : 'hidden'}
        animate="show"
      >
        <p className="mb-2 font-mono text-[10px] uppercase tracking-[0.2em] text-blue-400">Vault security controls</p>
        <h2 className="text-lg font-medium">Authenticated session</h2>
        <p className="mt-2 max-w-xl text-xs leading-6 text-slate-400">Manage the currently authenticated vault and its protected compartments. Locking the vault immediately removes access to these controls and protected contents.</p>
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
        {selectedNested && page === 'decrypt' && <section className="bv-premium-enter mt-6 rounded-xl border border-border bg-card p-5 transition-[border-color,box-shadow] duration-300 hover:border-sky-500/20 hover:shadow-[0_12px_40px_#00000018]">
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
      </motion.div>}
        </motion.div>
      </AnimatePresence>

      {error && <p role="alert" className="mt-4 rounded-lg border border-red-400/20 bg-red-400/5 px-4 py-3 text-sm text-red-300">{error}</p>}
      <p role="status" className="mt-4 flex items-center gap-2 text-xs text-slate-400">{notice && <><Check size={14} className="text-emerald-400" />{notice}</>}</p>
      <footer className="mt-8 flex flex-wrap justify-between gap-2 border-t border-border pt-5 font-mono text-[10px] uppercase tracking-widest text-slate-600"><span>Black Vault · Desktop foundation</span><span>{info ? `${info.platform} / ${info.architecture}` : 'Native connection pending'} · Auto-lock after {DEFAULT_IDLE_MS / 60000} min idle</span></footer>
    </main>
    </motion.div>
  </>;
}
