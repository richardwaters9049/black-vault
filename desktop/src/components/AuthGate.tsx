import { useEffect, useState, type FormEvent } from 'react';
import { ArrowUpRight, Eye, EyeOff, Fingerprint, HardDrive, KeyRound, LockKeyhole, Plus, ShieldCheck } from 'lucide-react';
import VaultMark from './VaultMark';
import { Button } from '@/components/ui/button';
import { errorMessage, native, type VaultSummary } from '@/lib/native';

interface Props { onAuthenticated: (vault: VaultSummary) => void }
export default function AuthGate({ onAuthenticated }: Props) {
  const [vaults, setVaults] = useState<VaultSummary[]>([]);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [creating, setCreating] = useState(false);
  const [selected, setSelected] = useState('');
  const [name, setName] = useState('');
  // Local development-only prefill; .env.development.local is ignored by Git.
  const [password, setPassword] = useState(import.meta.env.DEV ? (import.meta.env.VITE_DEV_VAULT_PASSWORD ?? '') : '');
  const [showPassword, setShowPassword] = useState(false);
  const [error, setError] = useState('');
  const [deleting, setDeleting] = useState(false);
  const [confirmation, setConfirmation] = useState('');
  // Copying text from a rendered instruction can introduce non-breaking spaces.
  const normalizedConfirmation = confirmation.replace(/[\u00a0\u202f]/g, ' ').trim();
  const confirmationMatches = normalizedConfirmation === `DELETE ${selected}`;

  useEffect(() => {
    let mounted = true;
    native.listVaults().then(items => {
      if (mounted) { setVaults(items); setSelected(items[0]?.id ?? ''); setCreating(items.length === 0); }
    }).catch(reason => { if (mounted) setError(errorMessage(reason)); })
      .finally(() => { if (mounted) setLoading(false); });
    return () => { mounted = false; };
  }, []);

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (busy || !password || (!creating && !selected)) return;
    setBusy(true); setError('');
    try {
      const vault = creating ? await native.createVault(name, password) : await native.unlockVault(selected, password);
      onAuthenticated(vault);
    } catch (reason) { setError(errorMessage(reason)); }
    finally { setPassword(''); setShowPassword(false); setBusy(false); }
  }

  async function deleteVault() {
    if (busy || !selected || !confirmationMatches) return;
    setBusy(true); setError('');
    try {
      await native.deleteLockedVault(selected, normalizedConfirmation);
      const remaining = await native.listVaults();
      setVaults(remaining); setSelected(remaining[0]?.id ?? ''); setCreating(remaining.length === 0);
      setDeleting(false); setConfirmation(''); setPassword('');
    } catch (reason) { setError(errorMessage(reason)); }
    finally { setBusy(false); }
  }

  return <main className="relative isolate overflow-hidden px-5 py-8 sm:px-8 sm:py-12 xl:px-12">
    <div aria-hidden="true" className="pointer-events-none absolute inset-0 -z-10 bg-[radial-gradient(ellipse_at_22%_34%,rgba(37,99,235,0.10),transparent_48%)]"/>
    <div className="mx-auto grid w-full max-w-[1320px] overflow-hidden rounded-2xl border border-blue-400/15 bg-[#09111c] shadow-[0_24px_85px_#0008] lg:grid-cols-[minmax(0,1.05fr)_minmax(0,0.95fr)]">
      <section aria-label="Black Vault introduction" className="relative flex flex-col overflow-hidden border-b border-blue-400/10 bg-[#080f19] p-7 sm:p-10 lg:border-r lg:border-b-0 lg:p-12">
        <div aria-hidden="true" className="pointer-events-none absolute inset-0 opacity-40 [background-image:linear-gradient(rgba(96,165,250,0.055)_1px,transparent_1px),linear-gradient(90deg,rgba(96,165,250,0.055)_1px,transparent_1px)] [background-size:38px_38px] [mask-image:linear-gradient(to_bottom,black,transparent_85%)]"/>
        <div className="relative flex items-center gap-3">
          <div className="grid size-11 place-items-center rounded-xl border border-blue-400/25 bg-blue-400/10"><VaultMark /></div>
          <div><p className="text-xs font-semibold tracking-[0.16em]">BLACK VAULT</p><p className="mt-1 font-mono text-[10px] uppercase tracking-widest text-slate-500">Private by design</p></div>
        </div>
        <div className="relative mt-12 max-w-lg lg:mt-24">
          <div className="mb-5 inline-flex items-center gap-2 rounded-full border border-sky-400/15 bg-sky-400/[0.06] px-3 py-1.5 font-mono text-[10px] uppercase tracking-[0.16em] text-sky-300"><span className="size-1.5 rounded-full bg-emerald-400"/> Offline-first security</div>
          <h2 className="max-w-md text-3xl font-medium leading-[1.17] tracking-tight text-slate-100 sm:text-4xl xl:text-[2.75rem]">Your files.<br/><span className="text-sky-300">Your keys.</span><br/>Your control.</h2>
          <p className="mt-6 max-w-sm text-sm leading-7 text-slate-400">A private space for your important documents, encrypted locally and accessible only through your vault credentials.</p>
        </div>
        <div className="relative mt-12 grid gap-3 sm:grid-cols-2 lg:mt-auto lg:pt-16">
          <div className="rounded-xl border border-white/[0.07] bg-white/[0.025] p-4">
            <LockKeyhole size={18} className="text-sky-300"/>
            <p className="mt-4 text-xs font-medium text-slate-200">Encrypted storage</p>
            <p className="mt-1.5 text-[11px] leading-5 text-slate-500">Protected vaults and nested compartments.</p>
          </div>
          <div className="rounded-xl border border-white/[0.07] bg-white/[0.025] p-4">
            <HardDrive size={18} className="text-sky-300"/>
            <p className="mt-4 text-xs font-medium text-slate-200">Stored on this Mac</p>
            <p className="mt-1.5 text-[11px] leading-5 text-slate-500">No online account or cloud sign-in required.</p>
          </div>
        </div>
        <div className="relative mt-6 flex items-center justify-between border-t border-white/[0.07] pt-5 font-mono text-[10px] uppercase tracking-[0.12em] text-slate-600"><span>Black Vault / Desktop</span><span className="flex items-center gap-1">Private workspace <ArrowUpRight size={12}/></span></div>
      </section>
      <section className="flex min-w-0 flex-col justify-center bg-[#0c1725] p-7 sm:p-10 lg:px-12 lg:py-14" aria-labelledby="auth-title">
      <div className="mb-8 flex items-center gap-3">
        <div className="grid size-10 place-items-center rounded-xl border border-sky-400/15 bg-sky-400/[0.06]"><Fingerprint size={20} className="text-sky-300"/></div>
        <div><p className="text-xs font-semibold tracking-[0.12em]">Secure access</p><p className="mt-1 font-mono text-[10px] uppercase tracking-widest text-slate-500">Local authentication</p></div>
      </div>
      <p className="mb-2 font-mono text-[10px] uppercase tracking-widest text-sky-400">{creating ? 'First-time setup' : 'Welcome back'}</p>
      <h1 id="auth-title" className="text-2xl font-medium">{creating ? 'Create your first vault' : 'Unlock your workspace'}</h1>
      <p className="mt-3 text-sm leading-6 text-slate-400">{creating ? 'Choose a name and a strong password. Your encryption key stays on this Mac.' : 'Select a vault and enter its password to access the dashboard.'}</p>
      {loading ? <p role="status" className="mt-8 text-sm text-slate-400">Checking local vaults…</p> :
        <form onSubmit={event => void submit(event)} className="mt-8 space-y-5">
          {creating ? <label className="block text-xs text-slate-400">Vault name
            <input aria-label="Vault name" value={name} onChange={event => setName(event.target.value)} maxLength={80} required disabled={busy}
              autoComplete="off" placeholder="Development Vault" className="mt-2 w-full rounded-lg border border-border bg-background px-3 py-3 text-sm text-slate-100 outline-none focus:border-sky-400" />
          </label> : <label className="block text-xs text-slate-400">Local vault
            <select aria-label="Local vault" value={selected} onChange={event => setSelected(event.target.value)} disabled={busy}
              className="mt-2 w-full rounded-lg border border-border bg-background px-3 py-3 text-sm text-slate-100 outline-none focus:border-sky-400">
              {vaults.map(vault => <option key={vault.id} value={vault.id}>{vault.name ?? 'Vault ' + vault.id.slice(0, 8)}</option>)}
            </select>
          </label>}
          <div className="text-xs text-slate-400">
            <label htmlFor="auth-vault-password">Vault password</label>
            <div className="relative mt-2">
              <input id="auth-vault-password" aria-label="Vault password" type={showPassword ? 'text' : 'password'} value={password} onChange={event => setPassword(event.target.value)}
                placeholder={creating ? 'Example: DevVault-Only-2026!' : 'Enter your existing vault password'} autoComplete={creating ? 'new-password' : 'current-password'} minLength={creating ? 12 : undefined} required disabled={busy}
                className="w-full rounded-lg border border-border bg-background px-3 py-3 pr-20 text-sm text-slate-100 outline-none focus:border-sky-400" />
              {creating && <button type="button" aria-label={showPassword ? 'Hide vault password' : 'Show vault password'} aria-pressed={showPassword} disabled={busy}
                onClick={() => setShowPassword(value => !value)} className="absolute inset-y-0 right-2 flex items-center gap-1.5 px-2 text-xs text-sky-400 hover:text-sky-200">
                {showPassword ? <EyeOff size={15} /> : <Eye size={15} />}{showPassword ? 'Hide' : 'Show'}
              </button>}
            </div>
            {creating && <p className="mt-2 text-[11px] leading-5 text-slate-500">Check what you typed before creating your vault. Hide it when someone can see your screen.</p>}
          </div>
          {error && <p role="alert" className="rounded-lg border border-red-400/20 bg-red-400/5 p-3 text-xs text-red-300">{error}</p>}
          <Button type="submit" disabled={busy || !password || (!creating && !selected)} className="w-full justify-center">
            {creating ? <Plus size={15} /> : <KeyRound size={15} />}{busy ? 'Please wait…' : creating ? 'Create and enter vault' : 'Unlock and sign in'}
          </Button>
          {!creating && selected && <div className="border-t border-border pt-4">
            <button type="button" disabled={busy} onClick={() => { setDeleting(value => !value); setConfirmation(''); }} className="text-xs text-red-300 hover:text-red-200">{deleting ? 'Cancel deletion' : 'Forgot password? / Delete vault'}</button>
            {deleting && <div className="mt-3 space-y-3 rounded-lg border border-red-400/25 bg-red-500/5 p-4">
              <p className="text-xs leading-5 text-red-200">No password recovery or reset exists. Deleting this vault permanently removes its local encrypted files and header. Backups or synced copies may remain elsewhere. This is not secure SSD erasure.</p>
              <label htmlFor="delete-vault-confirmation" className="block break-all font-mono text-[11px] text-slate-400">Type DELETE {selected} into the field below to confirm.</label>
              <input id="delete-vault-confirmation" type="text" aria-label="Delete vault confirmation" placeholder={`DELETE ${selected}`} spellCheck={false} autoCapitalize="off" autoCorrect="off" autoComplete="off" value={confirmation} onChange={event => setConfirmation(event.target.value)} disabled={busy} className="w-full rounded-lg border border-red-400/30 bg-background p-3 font-mono text-xs outline-none" />
              <p role="status" className={`text-[11px] ${confirmationMatches ? "text-emerald-300" : "text-slate-400"}`}>{confirmationMatches ? "Confirmation matched. You can now delete this vault." : "The delete button unlocks only when the full confirmation matches."}</p>
              <Button type="button" variant="outline" disabled={busy || !confirmationMatches} onClick={() => void deleteVault()}>Permanently delete selected vault</Button>
            </div>}
          </div>}
          {vaults.length > 0 && <button type="button" disabled={busy} onClick={() => { setCreating(value => !value); setError(''); setPassword(''); setShowPassword(false); }}
            className="w-full text-center text-xs text-sky-400 hover:text-sky-200">{creating ? 'Unlock an existing vault' : 'Create another vault'}</button>}
        </form>}
      <p className="mt-8 flex items-start gap-2 border-t border-border pt-5 text-xs leading-5 text-slate-500"><ShieldCheck size={15} className="mt-0.5 shrink-0 text-sky-400" />No online account or password recovery. Losing your vault password means losing access to its encrypted files.</p>
      </section>
    </div>
  </main>;
}
