import { useEffect, useState } from 'react';
import { ContactRound, FileLock2, KeyRound, Lock, LockOpen, ShieldCheck, ShieldQuestion } from 'lucide-react';
import { errorMessage, native, type TrustedContact, type VaultFile, type VaultSummary } from '@/lib/native';

export default function DashboardOverview({ currentVault, files }: { currentVault: VaultSummary; files: VaultFile[] }) {
  const [vaults, setVaults] = useState<VaultSummary[]>([]);
  const [contacts, setContacts] = useState<TrustedContact[]>([]);
  const [error, setError] = useState('');

  useEffect(() => {
    let active = true;
    Promise.all([native.listVaults(), native.listContacts()])
      .then(([localVaults, savedContacts]) => {
        if (!active) return;
        setVaults(localVaults);
        setContacts(savedContacts);
      })
      .catch(reason => { if (active) setError(errorMessage(reason)); });
    return () => { active = false; };
  }, [currentVault.id]);

  const verified = contacts.filter(contact => contact.trust === 'verified').length;
  const changed = contacts.filter(contact => contact.trust === 'key_changed').length;

  return <div className="bv-page-enter pt-6">
    <p className="mb-3 font-mono text-[10px] uppercase tracking-[0.2em] text-blue-400">Your private workspace</p>
    <h1 className="text-2xl font-medium tracking-tight sm:text-3xl">Overview</h1>
    <p className="mt-2 max-w-2xl text-xs leading-6 text-slate-400">
      A local snapshot of your vault and trusted-contact state. Encryption and decryption now have dedicated workspaces.
    </p>

    {error && <p role="alert" className="mt-5 rounded-lg border border-red-400/20 bg-red-500/[0.06] px-4 py-3 text-xs text-red-200">{error}</p>}

    <div className="mt-7 grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
      <div className="rounded-xl border border-border bg-card p-5">
        <KeyRound size={17} className="text-blue-300"/>
        <p className="mt-4 text-2xl font-medium">{vaults.length}</p>
        <p className="mt-1 text-[11px] text-slate-500">Local vault{vaults.length === 1 ? '' : 's'}</p>
      </div>
      <div className="rounded-xl border border-border bg-card p-5">
        <FileLock2 size={17} className="text-sky-300"/>
        <p className="mt-4 text-2xl font-medium">{files.length}</p>
        <p className="mt-1 text-[11px] text-slate-500">Encrypted file{files.length === 1 ? '' : 's'} in current vault</p>
      </div>
      <div className="rounded-xl border border-border bg-card p-5">
        <ShieldCheck size={17} className="text-emerald-300"/>
        <p className="mt-4 text-2xl font-medium">{verified}</p>
        <p className="mt-1 text-[11px] text-slate-500">Verified contact{verified === 1 ? '' : 's'}</p>
      </div>
      <div className="rounded-xl border border-border bg-card p-5">
        <ShieldQuestion size={17} className={changed ? 'text-red-300' : 'text-slate-500'}/>
        <p className="mt-4 text-2xl font-medium">{changed}</p>
        <p className="mt-1 text-[11px] text-slate-500">Contact key change{changed === 1 ? '' : 's'} requiring review</p>
      </div>
    </div>

    <div className="mt-6 grid gap-5 lg:grid-cols-2">
      <section className="rounded-xl border border-border bg-card p-5 sm:p-6">
        <div className="flex items-center gap-2"><KeyRound size={15} className="text-blue-300"/><h2 className="text-sm font-medium">Vault snapshot</h2></div>
        <p className="mt-2 text-[11px] leading-5 text-slate-500">Vault creation timestamps are not currently persisted, so this list does not claim chronological recency.</p>
        <div className="mt-4 space-y-2">
          {vaults.slice(0, 5).map(vault => <div key={vault.id} className="flex items-center gap-3 rounded-lg border border-border bg-black/15 p-3">
            {vault.isLocked ? <Lock size={14} className="text-slate-500"/> : <LockOpen size={14} className="text-emerald-300"/>}
            <div className="min-w-0 flex-1">
              <p className="truncate text-xs text-slate-200">{vault.name ?? 'Locked vault'}</p>
              <p className="mt-1 font-mono text-[10px] text-slate-600">{vault.id.slice(0, 16)}…</p>
            </div>
            <span className="text-[10px] text-slate-500">{vault.isLocked ? 'Locked' : 'Open'}</span>
          </div>)}
          {vaults.length === 0 && <p className="py-5 text-xs text-slate-500">No local vaults found.</p>}
        </div>
      </section>

      <section className="rounded-xl border border-border bg-card p-5 sm:p-6">
        <div className="flex items-center gap-2"><ContactRound size={15} className="text-blue-300"/><h2 className="text-sm font-medium">Contact snapshot</h2></div>
        <p className="mt-2 text-[11px] leading-5 text-slate-500">Contact-added timestamps are not currently persisted. Trust state below is current, not a fabricated activity history.</p>
        <div className="mt-4 space-y-2">
          {contacts.slice(0, 5).map(contact => <div key={contact.name} className="flex items-center gap-3 rounded-lg border border-border bg-black/15 p-3">
            <ContactRound size={14} className={contact.trust === 'verified' ? 'text-emerald-300' : contact.trust === 'key_changed' ? 'text-red-300' : 'text-slate-500'}/>
            <p className="min-w-0 flex-1 truncate text-xs text-slate-200">{contact.name}</p>
            <span className="font-mono text-[10px] uppercase text-slate-500">{contact.trust.replace('_', ' ')}</span>
          </div>)}
          {contacts.length === 0 && <p className="py-5 text-xs text-slate-500">No contacts imported yet.</p>}
        </div>
      </section>
    </div>
  </div>;
}
