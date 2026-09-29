import { useEffect, useState, type FormEvent } from 'react';
import { ArrowRight, Eye, EyeOff, KeyRound, Plus } from 'lucide-react';
import VaultMark from './VaultMark';
import AuthGlobe from './AuthGlobe';
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

  return <main className="bv-auth relative isolate grid w-full place-items-center overflow-hidden px-4 py-4 sm:px-6 sm:py-6">
    <div className="bv-auth-ambient pointer-events-none absolute inset-0" aria-hidden="true" />
    <div className="bv-auth-stars pointer-events-none absolute inset-0" aria-hidden="true" />
    <div className="bv-auth-horizon pointer-events-none absolute inset-x-0 bottom-0" aria-hidden="true" />
    <div className="bv-globe-scene bv-globe-enter pointer-events-none absolute left-1/2 top-1/2" aria-hidden="true">
      <AuthGlobe />
      <span className="bv-globe-orbit bv-globe-orbit-one" />
      <span className="bv-globe-orbit bv-globe-orbit-two" />
      <span className="bv-globe-orbit bv-globe-orbit-three" />
      <span className="bv-orbit-point bv-orbit-point-one" />
      <span className="bv-orbit-point bv-orbit-point-two" />
      <span className="bv-orbit-point bv-orbit-point-three" />
    </div>
    <section className="bv-auth-panel bv-auth-panel-enter relative z-10 rounded-[22px] border border-blue-300/20 bg-[#0c1420]/95 shadow-[0_30px_110px_#000b] backdrop-blur-xl" aria-labelledby="auth-title">
      <div className="mb-9 flex flex-col items-center text-center">
        <div className="mb-5 grid size-[70px] place-items-center rounded-2xl border border-blue-400/20 bg-blue-400/[0.045] shadow-[0_0_38px_#2563eb15]">
          <VaultMark size="large" />
        </div>
        <p className="text-[18px] font-semibold tracking-[0.32em] text-slate-100">BLACK <span className="text-sky-400">VAULT</span></p>
        <p className="mt-2 text-[10px] uppercase tracking-[0.22em] text-slate-500">Your private space, offline.</p>
      </div>
      <h1 id="auth-title" className="mb-2 text-center text-lg font-medium text-slate-100">{creating ? 'Create your first vault' : 'Unlock your workspace'}</h1>
      <p className="text-center text-xs leading-5 text-slate-400">{creating ? 'Choose a vault name and a strong password.' : 'Enter your password to continue.'}</p>
      {loading ? <p role="status" className="mt-8 text-sm text-slate-400">Checking local vaults…</p> :
        <form onSubmit={event => void submit(event)} className="mt-7 space-y-4">
          {creating ? <label className="block text-xs text-slate-400">Vault name
            <input aria-label="Vault name" value={name} onChange={event => setName(event.target.value)} maxLength={80} required disabled={busy}
              autoComplete="off" placeholder="Development Vault" className="mt-2 w-full rounded-xl border border-blue-300/15 bg-[#080e18] px-3 py-3 text-sm text-slate-100 outline-none focus:border-sky-400" />
          </label> : <label className="block text-xs text-slate-400">Local vault
            <select aria-label="Local vault" value={selected} onChange={event => setSelected(event.target.value)} disabled={busy}
              className="mt-2 w-full rounded-xl border border-blue-300/15 bg-[#080e18] px-3 py-3 text-sm text-slate-100 outline-none focus:border-sky-400">
              {vaults.map(vault => <option key={vault.id} value={vault.id}>{vault.name ?? 'Vault ' + vault.id.slice(0, 8)}</option>)}
            </select>
          </label>}
          <div className="text-xs text-slate-400">
            <label htmlFor="auth-vault-password">Vault password</label>
            <div className="relative mt-2">
              <input id="auth-vault-password" aria-label="Vault password" type={showPassword ? 'text' : 'password'} value={password} onChange={event => setPassword(event.target.value)}
                placeholder={creating ? 'Example: DevVault-Only-2026!' : 'Enter your existing vault password'} autoComplete={creating ? 'new-password' : 'current-password'} minLength={creating ? 12 : undefined} required disabled={busy}
                className="w-full rounded-xl border border-blue-300/15 bg-[#080e18] px-3 py-3 pr-20 text-sm text-slate-100 outline-none focus:border-sky-400" />
              {<button type="button" aria-label={showPassword ? 'Hide vault password' : 'Show vault password'} aria-pressed={showPassword} disabled={busy}
                onClick={() => setShowPassword(value => !value)} className="absolute inset-y-0 right-2 flex items-center gap-1.5 px-2 text-xs text-sky-400 hover:text-sky-200">
                {showPassword ? <EyeOff size={15} /> : <Eye size={15} />}{showPassword ? 'Hide' : 'Show'}
              </button>}
            </div>
            {creating && <p className="mt-2 text-[11px] leading-5 text-slate-500">Check what you typed before creating your vault. Hide it when someone can see your screen.</p>}
          </div>
          {error && <p role="alert" className="rounded-lg border border-red-400/20 bg-red-400/5 p-3 text-xs text-red-300">{error}</p>}
          <Button type="submit" disabled={busy || !password || (!creating && !selected)} className="w-full justify-center rounded-xl bg-gradient-to-r from-blue-600 to-sky-500 py-3.5 shadow-[0_6px_28px_#2563eb24] hover:from-blue-500 hover:to-sky-400">
            {creating ? <Plus size={15} /> : <KeyRound size={15} />}{busy ? 'Please wait…' : creating ? 'Create vault' : 'Unlock vault'}{!busy && <ArrowRight size={15} />}
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
            className="w-full rounded-xl border border-blue-300/15 bg-white/[0.025] py-3 text-center text-xs text-sky-300 hover:border-blue-300/35 hover:text-sky-100">{creating ? 'Unlock an existing vault' : 'Create another vault'}</button>}
        </form>}
      <p className="mt-6 border-t border-white/[0.06] pt-5 text-center text-[11px] leading-5 text-slate-500">No password recovery. Keep your credentials somewhere safe.</p>
    </section>
  </main>;
}
