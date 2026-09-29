import { useEffect, useState, type FormEvent } from 'react';
import { KeyRound, Lock, LockOpen, Plus, ShieldCheck } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { errorMessage, native, type VaultSummary } from '@/lib/native';

interface Props { ready: boolean }

export default function VaultWorkspace({ ready }: Props) {
  const [vaults, setVaults] = useState<VaultSummary[]>([]);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [name, setName] = useState('');
  const [password, setPassword] = useState('');
  const [unlockId, setUnlockId] = useState<string | null>(null);
  const [unlockPassword, setUnlockPassword] = useState('');
  const [error, setError] = useState('');
  const [notice, setNotice] = useState('');
  const active = vaults.find(vault => !vault.isLocked);

  useEffect(() => {
    if (!ready) return;
    let mounted = true;
    native.listVaults().then(items => { if (mounted) setVaults(items); })
      .catch(reason => { if (mounted) setError(errorMessage(reason)); })
      .finally(() => { if (mounted) setLoading(false); });
    return () => { mounted = false; };
  }, [ready]);

  async function refresh() { setVaults(await native.listVaults()); }

  async function create(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (busy || active) return;
    setBusy(true); setError(''); setNotice('');
    try {
      await native.createVault(name, password);
      await refresh();
      setName('');
      setNotice('Encrypted vault header saved on this Mac. The vault is now unlocked.');
    } catch (reason) { setError(errorMessage(reason)); }
    finally { setPassword(''); setBusy(false); }
  }

  async function unlock(event: FormEvent<HTMLFormElement>, vaultId: string) {
    event.preventDefault();
    if (busy || active) return;
    setBusy(true); setError(''); setNotice('');
    try {
      await native.unlockVault(vaultId, unlockPassword);
      await refresh();
      setUnlockId(null);
      setNotice('Vault unlocked. The master key remains in the Rust process.');
    } catch (reason) { setError(errorMessage(reason)); }
    finally { setUnlockPassword(''); setBusy(false); }
  }

  async function lock() {
    if (busy) return;
    setBusy(true); setError(''); setNotice('');
    try {
      await native.lockVault();
      await refresh();
      setNotice('Vault locked. The in-memory master key was released.');
    } catch (reason) { setError(errorMessage(reason)); }
    finally { setBusy(false); }
  }

  return <section aria-labelledby="vault-workspace-title" className="mt-8 overflow-hidden rounded-xl border border-border bg-card">
    <div className="flex flex-wrap items-center justify-between gap-3 border-b border-border px-6 py-5">
      <div>
        <h2 id="vault-workspace-title" className="flex items-center gap-2 text-sm font-medium"><KeyRound size={16} className="text-blue-400" /> Vault workspace</h2>
        <p className="mt-1 text-xs text-slate-500">Offline vault creation, password-based unlock and explicit locking.</p>
      </div>
      <span className="rounded-full border border-blue-400/20 bg-blue-500/5 px-2.5 py-1 font-mono text-[10px] uppercase tracking-widest text-blue-300">Sprint 02 foundation</span>
    </div>
    <div className="grid gap-6 p-6 lg:grid-cols-[1.1fr_1fr]">
      <div>
        {active ? <>
          <p className="text-xs uppercase tracking-widest text-emerald-400">Unlocked on this Mac</p>
          <h3 className="mt-3 break-words text-xl font-medium">{active.name}</h3>
          <p className="mt-2 font-mono text-[11px] text-slate-500">Vault ID: {active.id}</p>
          <p className="mt-4 max-w-sm text-xs leading-6 text-slate-400">The vault master key is held only by Rust for this session. Lock before opening another vault.</p>
          <Button className="mt-5" variant="outline" onClick={() => void lock()} disabled={busy}><Lock size={15} />{busy ? 'Locking…' : 'Lock vault'}</Button>
        </> : <>
          <h3 className="flex items-center gap-2 text-sm font-medium"><Plus size={16} /> Create a local vault</h3>
          <p className="mt-2 max-w-md text-xs leading-6 text-slate-400">Each vault receives a fresh random 256-bit master key, wrapped using Argon2id and AES-256-GCM. Its display name is encrypted too.</p>
          <form onSubmit={event => void create(event)} className="mt-5 space-y-3">
            <label htmlFor="vault-name" className="block text-xs text-slate-300">Vault name</label>
            <input id="vault-name" value={name} onChange={event => setName(event.target.value)}
              maxLength={80} required disabled={busy || !ready}
              placeholder="e.g. Personal" autoComplete="off"
              className="w-full rounded-lg border border-border bg-background px-3 py-2.5 text-sm outline-none focus:border-blue-400" />
            <label htmlFor="vault-password" className="block text-xs text-slate-300">Vault password (at least 12 characters)</label>
            <input id="vault-password" type="password" value={password} onChange={event => setPassword(event.target.value)}
              minLength={12} maxLength={1024} required disabled={busy || !ready}
              autoComplete="new-password"
              className="w-full rounded-lg border border-border bg-background px-3 py-2.5 text-sm outline-none focus:border-blue-400" />
            <Button type="submit" disabled={busy || !ready}>{busy ? 'Creating…' : 'Create encrypted vault'}<LockOpen size={15} /></Button>
          </form>
        </>}
      </div>
      <div className="rounded-lg border border-border bg-[#0b111b] p-4">
        <h3 className="text-xs font-medium text-slate-200">Local vaults</h3>
        <p className="mt-1 text-xs leading-5 text-slate-500">Locked names are hidden. Identifiers are not secret.</p>
        {!ready || loading ? <p className="mt-5 text-xs text-slate-500">Checking local vaults…</p>
          : vaults.length === 0 ? <p className="mt-5 text-xs text-slate-500">No local vaults yet.</p>
          : <div className="mt-4 space-y-3">{vaults.map(vault => <div key={vault.id} className="rounded-lg border border-border p-3">
            <div className="flex items-center gap-2 text-sm">{vault.isLocked ? <Lock size={14} className="text-slate-500" /> : <LockOpen size={14} className="text-emerald-400" />}
              <span className="min-w-0 flex-1 break-words">{vault.name ?? 'Vault ' + vault.id.slice(0, 8)}</span>
              <span className="font-mono text-[10px] text-slate-500">{vault.isLocked ? 'Locked' : 'Open'}</span>
            </div>
            {vault.isLocked && !active && <div className="mt-3">
              {unlockId === vault.id ? <form onSubmit={event => void unlock(event, vault.id)} className="space-y-2">
                <label htmlFor={'unlock-' + vault.id} className="block text-xs text-slate-400">Password</label>
                <input id={'unlock-' + vault.id} type="password" value={unlockPassword}
                  onChange={event => setUnlockPassword(event.target.value)} autoComplete="current-password"
                  required disabled={busy} className="w-full rounded-lg border border-border bg-background px-3 py-2 text-sm outline-none focus:border-blue-400" />
                <div className="flex flex-wrap gap-2">
                  <Button type="submit" disabled={busy}>{busy ? 'Unlocking…' : 'Unlock vault'}</Button>
                  <Button variant="outline" disabled={busy} onClick={() => { setUnlockId(null); setUnlockPassword(''); }}>Cancel</Button>
                </div>
              </form> : <Button variant="outline" disabled={busy} onClick={() => { setUnlockId(vault.id); setUnlockPassword(''); }}>Unlock</Button>}
            </div>}
          </div>)}</div>}
      </div>
    </div>
    {error && <p role="alert" className="mx-6 mb-4 rounded-lg border border-red-400/20 bg-red-400/5 px-4 py-3 text-sm text-red-300">{error}</p>}
    {notice && <p role="status" className="mx-6 mb-4 rounded-lg border border-emerald-400/20 bg-emerald-400/5 px-4 py-3 text-xs text-emerald-300">{notice}</p>}
    <div className="flex items-start gap-2 border-t border-border bg-[#0b111b] px-6 py-3.5 text-xs leading-5 text-slate-400">
      <ShieldCheck size={14} className="mt-0.5 shrink-0 text-blue-400" />
      This stage saves an authenticated encrypted vault header. File import, encrypted file storage and recovery are not implemented yet.
    </div>
  </section>;
}
