import { useEffect, useState, type FormEvent } from 'react';
import { Eye, EyeOff, KeyRound, Plus, ShieldCheck } from 'lucide-react';
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
  const [password, setPassword] = useState('');
  const [showPassword, setShowPassword] = useState(false);
  const [error, setError] = useState('');

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

  return <main className="grid place-items-center px-5 py-14">
    <section className="w-full max-w-md rounded-2xl border border-blue-400/15 bg-[#091522] p-7 shadow-[0_24px_85px_#0008] sm:p-9" aria-labelledby="auth-title">
      <div className="mb-8 flex items-center gap-3">
        <div className="grid size-11 place-items-center rounded-xl border border-blue-400/25 bg-blue-400/10"><VaultMark /></div>
        <div><p className="text-xs font-semibold tracking-[0.16em]">BLACK VAULT</p><p className="mt-1 font-mono text-[10px] uppercase tracking-widest text-slate-500">Local authentication</p></div>
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
          {vaults.length > 0 && <button type="button" disabled={busy} onClick={() => { setCreating(value => !value); setError(''); setPassword(''); setShowPassword(false); }}
            className="w-full text-center text-xs text-sky-400 hover:text-sky-200">{creating ? 'Unlock an existing vault' : 'Create another vault'}</button>}
        </form>}
      <p className="mt-8 flex items-start gap-2 border-t border-border pt-5 text-xs leading-5 text-slate-500"><ShieldCheck size={15} className="mt-0.5 shrink-0 text-sky-400" />No online account or password recovery. Losing your vault password means losing access to its encrypted files.</p>
    </section>
  </main>;
}
