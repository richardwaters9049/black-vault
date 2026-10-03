import { useEffect, useState, type FormEvent } from 'react';
import { ArrowRight, Eye, EyeOff, KeyRound, Lock, Plus, ShieldCheck } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { errorMessage, native, type VaultSummary } from '@/lib/native';

export default function VaultAccess({ onAuthenticated }: { onAuthenticated: (vault: VaultSummary) => void }) {
  const [vaults, setVaults] = useState<VaultSummary[]>([]);
  const [selected, setSelected] = useState('');
  const [creating, setCreating] = useState(false);
  const [name, setName] = useState('');
  const [password, setPassword] = useState(import.meta.env.DEV ? (import.meta.env.VITE_DEV_VAULT_PASSWORD ?? '') : '');
  const [showPassword, setShowPassword] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');

  useEffect(() => {
    let active = true;
    native.listVaults().then(items => {
      if (!active) return;
      setVaults(items);
      setSelected(items[0]?.id ?? '');
      setCreating(items.length === 0);
    }).catch(reason => { if (active) setError(errorMessage(reason)); });
    return () => { active = false; };
  }, []);

  async function submit(event: FormEvent) {
    event.preventDefault();
    if (busy || !password || (creating ? !name.trim() : !selected)) return;
    setBusy(true); setError('');
    try {
      const vault = creating
        ? await native.createVault(name.trim(), password)
        : await native.unlockVault(selected, password);
      onAuthenticated(vault);
    } catch (reason) {
      setError(errorMessage(reason));
    } finally {
      setPassword(''); setShowPassword(false); setBusy(false);
    }
  }

  return <section aria-labelledby="vault-access-title" className="bv-page-enter mt-6 overflow-hidden rounded-xl border border-blue-400/15 bg-card">
    <div className="border-b border-border bg-blue-500/[0.035] px-5 py-5 sm:px-6">
      <div className="flex items-center gap-3">
        <div className="flex size-10 items-center justify-center rounded-lg border border-blue-400/20 bg-blue-500/10 text-blue-300"><Lock size={17}/></div>
        <div>
          <h2 id="vault-access-title" className="text-sm font-medium">{creating ? 'Create a protected vault' : 'Authenticate to open a vault'}</h2>
          <p className="mt-1 text-[11px] leading-5 text-slate-500">Protected filenames and contents remain unavailable until native vault authentication succeeds.</p>
        </div>
      </div>
    </div>

    <form onSubmit={event => void submit(event)} className="p-5 sm:p-6">
      {!creating && <label className="block text-xs text-slate-400">Locked vault
        <select aria-label="Locked vault" value={selected} onChange={event => setSelected(event.target.value)} disabled={busy}
          className="mt-2 w-full rounded-lg border border-border bg-background px-3 py-3 text-sm text-slate-100 outline-none focus:border-sky-400">
          {vaults.map(vault => <option key={vault.id} value={vault.id}>Vault {vault.id.slice(0, 8)}</option>)}
        </select>
      </label>}

      {creating && <label className="block text-xs text-slate-400">Vault name
        <input aria-label="Vault name" value={name} onChange={event => setName(event.target.value)} maxLength={80} required disabled={busy}
          autoComplete="off" placeholder="Personal" className="mt-2 w-full rounded-lg border border-border bg-background px-3 py-3 text-sm text-slate-100 outline-none focus:border-sky-400"/>
      </label>}

      <label className="mt-4 block text-xs text-slate-400">Vault password
        <div className="relative mt-2">
          <input aria-label="Vault password" type={showPassword ? 'text' : 'password'} value={password} onChange={event => setPassword(event.target.value)}
            minLength={creating ? 12 : undefined} required disabled={busy} autoComplete={creating ? 'new-password' : 'current-password'}
            className="w-full rounded-lg border border-border bg-background px-3 py-3 pr-20 text-sm text-slate-100 outline-none focus:border-sky-400"/>
          <button type="button" aria-label={showPassword ? 'Hide vault password' : 'Show vault password'} onClick={() => setShowPassword(value => !value)}
            className="absolute inset-y-0 right-2 flex items-center gap-1.5 px-2 text-xs text-sky-400">
            {showPassword ? <EyeOff size={14}/> : <Eye size={14}/>} {showPassword ? 'Hide' : 'Show'}
          </button>
        </div>
      </label>

      {error && <p role="alert" className="mt-4 rounded-lg border border-red-400/20 bg-red-500/[0.06] p-3 text-xs text-red-200">{error}</p>}

      <div className="mt-5 flex flex-wrap gap-2">
        <Button type="submit" disabled={busy || !password || (creating ? !name.trim() : !selected)}>
          {creating ? <Plus size={14}/> : <KeyRound size={14}/>}
          {busy ? 'Authenticating…' : creating ? 'Create & open vault' : 'Authenticate & open'}
          {!busy && <ArrowRight size={14}/>}
        </Button>
        {vaults.length > 0 && <Button type="button" variant="outline" disabled={busy} onClick={() => { setCreating(value => !value); setPassword(''); setError(''); }}>
          {creating ? 'Choose existing vault' : 'Create another vault'}
        </Button>}
      </div>
    </form>

    <div className="flex items-start gap-2 border-t border-border bg-black/15 px-5 py-4 text-[11px] leading-5 text-slate-500 sm:px-6">
      <ShieldCheck size={14} className="mt-0.5 shrink-0 text-blue-300"/>
      Password authentication remains the cryptographic fallback. Desktop OS biometric authentication will be added at this native boundary; Black Vault will not collect fingerprint data itself.
    </div>
  </section>;
}
