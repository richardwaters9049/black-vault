import { useEffect, useState } from 'react';
import { motion, useReducedMotion, type Variants } from 'framer-motion';
import { ArrowRight, ContactRound, FileLock2, KeyRound, Lock, LockOpen, ShieldCheck, ShieldQuestion } from 'lucide-react';
import VaultHero from '@/components/VaultHero';
import { errorMessage, native, type TrustedContact, type VaultFile, type VaultSummary } from '@/lib/native';

const overviewContainer: Variants = {
  hidden: {},
  show: { transition: { staggerChildren: 0.08, delayChildren: 0.04 } },
};

const overviewItem: Variants = {
  hidden: { opacity: 0, y: 14, scale: 0.995 },
  show: { opacity: 1, y: 0, scale: 1, transition: { duration: 0.34, ease: [0.22, 1, 0.36, 1] } },
};

const cardContainer: Variants = {
  hidden: {},
  show: { transition: { staggerChildren: 0.055 } },
};

export default function DashboardOverview({
  currentVault,
  files,
  onCreateVault,
}: {
  currentVault: VaultSummary | null;
  files: VaultFile[];
  onCreateVault: (intent: 'encrypt' | 'decrypt') => void;
}) {
  const [choosingVaultType, setChoosingVaultType] = useState(false);
  const [vaults, setVaults] = useState<VaultSummary[]>([]);
  const [contacts, setContacts] = useState<TrustedContact[]>([]);
  const [error, setError] = useState('');
  const [vaultState, setVaultState] = useState<'loading' | 'ready' | 'error'>('loading');
  const [contactState, setContactState] = useState<'loading' | 'ready' | 'error'>('loading');
  const reduceMotion = useReducedMotion();

  useEffect(() => {
    let active = true;
    setError(''); setVaultState('loading'); setContactState('loading');
    // Independent reads keep a contact-store failure from hiding available vaults.
    native.listVaults().then(items => {
      if (active) { setVaults(items); setVaultState('ready'); }
    }).catch(reason => { if (active) { setVaultState('error'); setError(current => [current, errorMessage(reason)].filter(Boolean).join(' ')); } });
    native.listContacts().then(items => {
      if (active) { setContacts(items); setContactState('ready'); }
    }).catch(reason => { if (active) { setContactState('error'); setError(current => [current, errorMessage(reason)].filter(Boolean).join(' ')); } });
    return () => { active = false; };
  }, [currentVault?.id]);

  const verified = contacts.filter(contact => contact.trust === 'verified').length;
  const changed = contacts.filter(contact => contact.trust === 'key_changed').length;
  const initial = reduceMotion ? false : 'hidden';

  return <motion.div
    className="pt-6"
    variants={overviewContainer}
    initial={initial}
    animate="show"
  >
    <motion.div variants={overviewItem}>
      <p className="mb-3 font-mono text-[10px] uppercase tracking-[0.2em] text-blue-400">Your private workspace</p>
      <h1 className="text-2xl font-medium tracking-tight sm:text-3xl">Overview</h1>
      <p className="mt-2 max-w-2xl text-xs leading-6 text-slate-400">
        A local snapshot of your vault and trusted-contact state. Encryption and decryption now have dedicated workspaces.
      </p>
    </motion.div>

    {error && <motion.p variants={overviewItem} role="alert" className="mt-5 rounded-lg border border-red-400/20 bg-red-500/[0.06] px-4 py-3 text-xs text-red-200">{error}</motion.p>}

    {vaultState === 'ready' && vaults.length === 0 && <motion.section variants={overviewItem} className="bv-empty-vault mt-7 overflow-hidden rounded-2xl border border-blue-400/15 bg-card">
      <motion.div variants={cardContainer} className="grid items-center gap-4 lg:grid-cols-[1.08fr_.92fr]">
        <motion.div variants={overviewItem} className="px-6 pb-2 pt-7 sm:px-8 lg:py-9">
          <p className="font-mono text-[10px] uppercase tracking-[0.2em] text-sky-400">Your encrypted world</p>
          <h2 className="mt-3 text-xl font-medium tracking-tight sm:text-2xl">Create your first vault</h2>
          <p className="mt-3 max-w-lg text-sm leading-7 text-slate-400">Build a private encrypted workspace on this device. Vault contents stay hidden until you authenticate.</p>
          <button type="button" onClick={() => setChoosingVaultType(true)}
            className="mt-5 inline-flex items-center gap-2 rounded-lg border border-blue-400/20 bg-blue-500/10 px-4 py-2.5 text-sm font-medium text-blue-200 hover:bg-blue-500/15 hover:text-white">
            Create a vault <ArrowRight size={15}/>
          </button>
        </motion.div>
        <motion.div variants={overviewItem}><VaultHero/></motion.div>
      </motion.div>
    </motion.section>}

    {choosingVaultType && <div className="fixed inset-0 z-50 grid place-items-center bg-black/65 p-5 backdrop-blur-sm" role="dialog" aria-modal="true" aria-labelledby="vault-type-title" onMouseDown={event => { if (event.currentTarget === event.target) setChoosingVaultType(false); }}>
      <div className="bv-dialog-enter w-full max-w-2xl rounded-2xl border border-blue-400/20 bg-[#0b111b] p-6 shadow-[0_24px_90px_#000a] sm:p-7">
        <p className="font-mono text-[10px] uppercase tracking-[0.2em] text-sky-400">New vault</p>
        <h2 id="vault-type-title" className="mt-2 text-xl font-medium">What do you want to do?</h2>
        <p className="mt-2 text-sm leading-7 text-slate-400">Both choices create the same secure local vault. Your choice opens the workspace for the task you want to perform first.</p>
        <div className="mt-6 grid gap-3 sm:grid-cols-2">
          <button type="button" onClick={() => onCreateVault('encrypt')} className="group rounded-xl border border-blue-400/15 bg-blue-500/[0.05] p-5 text-left transition hover:border-blue-400/35 hover:bg-blue-500/10">
            <FileLock2 size={20} className="text-sky-300"/>
            <span className="mt-4 block text-base font-medium text-slate-100">Encryption Vault</span>
            <span className="mt-2 block text-xs leading-6 text-slate-400">Create the vault, then protect files by encrypting them into it.</span>
            <span className="mt-4 flex items-center gap-1.5 text-xs text-blue-300">Choose encryption <ArrowRight size={13}/></span>
          </button>
          <button type="button" onClick={() => onCreateVault('decrypt')} className="group rounded-xl border border-blue-400/15 bg-blue-500/[0.05] p-5 text-left transition hover:border-blue-400/35 hover:bg-blue-500/10">
            <LockOpen size={20} className="text-blue-300"/>
            <span className="mt-4 block text-base font-medium text-slate-100">Decryption Vault</span>
            <span className="mt-2 block text-xs leading-6 text-slate-400">Create the vault, then open, examine and edit protected content.</span>
            <span className="mt-4 flex items-center gap-1.5 text-xs text-blue-300">Choose decryption <ArrowRight size={13}/></span>
          </button>
        </div>
        <button type="button" onClick={() => setChoosingVaultType(false)} className="mt-5 text-xs text-slate-500 hover:text-slate-300">Cancel</button>
      </div>
    </div>}

    <motion.div variants={cardContainer} className="mt-7 grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
      <motion.div variants={overviewItem} className="rounded-xl border border-border bg-card p-5">
        <KeyRound size={17} className="text-blue-300"/>
        <p className="mt-4 text-2xl font-medium">{vaultState === 'ready' ? vaults.length : '—'}</p>
        <p className="mt-1 text-[11px] text-slate-500">Local vault{vaults.length === 1 ? '' : 's'}</p>
      </motion.div>
      <motion.div variants={overviewItem} className="rounded-xl border border-border bg-card p-5">
        <FileLock2 size={17} className="text-sky-300"/>
        <p className="mt-4 text-2xl font-medium">{currentVault ? files.length : '—'}</p>
        <p className="mt-1 text-[11px] text-slate-500">{currentVault ? `Encrypted file${files.length === 1 ? '' : 's'} in authenticated vault` : 'Vault contents hidden while locked'}</p>
      </motion.div>
      <motion.div variants={overviewItem} className="rounded-xl border border-border bg-card p-5">
        <ShieldCheck size={17} className="text-emerald-300"/>
        <p className="mt-4 text-2xl font-medium">{contactState === 'ready' ? verified : '—'}</p>
        <p className="mt-1 text-[11px] text-slate-500">Verified contact{verified === 1 ? '' : 's'}</p>
      </motion.div>
      <motion.div variants={overviewItem} className="rounded-xl border border-border bg-card p-5">
        <ShieldQuestion size={17} className={changed ? 'text-red-300' : 'text-slate-500'}/>
        <p className="mt-4 text-2xl font-medium">{contactState === 'ready' ? changed : '—'}</p>
        <p className="mt-1 text-[11px] text-slate-500">Contact key change{changed === 1 ? '' : 's'} requiring review</p>
      </motion.div>
    </motion.div>

    <motion.div variants={cardContainer} className="mt-6 grid gap-5 lg:grid-cols-2">
      <motion.section variants={overviewItem} className="rounded-xl border border-border bg-card p-5 sm:p-6">
        <div className="flex items-center gap-2"><KeyRound size={15} className="text-blue-300"/><h2 className="text-sm font-medium">Vault snapshot</h2></div>
        <p className="mt-2 text-[11px] leading-5 text-slate-500">Vault creation timestamps are not currently persisted, so this list does not claim chronological recency.</p>
        <div className="mt-4 space-y-2">
          {vaultState !== 'ready' && <p className="py-5 text-xs text-slate-500">{vaultState === 'loading' ? 'Loading vaults…' : 'Vault list unavailable.'}</p>}
          {(vaultState === 'ready' ? vaults : []).slice(0, 5).map(vault => <div key={vault.id} className="flex items-center gap-3 rounded-lg border border-border bg-black/15 p-3">
            {vault.isLocked ? <Lock size={14} className="text-slate-500"/> : <LockOpen size={14} className="text-emerald-300"/>}
            <div className="min-w-0 flex-1">
              <p className="truncate text-xs text-slate-200">{vault.name ?? 'Locked vault'}</p>
              <p className="mt-1 font-mono text-[10px] text-slate-600">{vault.id.slice(0, 16)}…</p>
            </div>
            <span className="text-[10px] text-slate-500">{vault.isLocked ? 'Locked' : 'Open'}</span>
          </div>)}
          {vaultState === 'ready' && vaults.length === 0 && <p className="py-5 text-xs text-slate-500">No local vaults found.</p>}
        </div>
      </motion.section>

      <motion.section variants={overviewItem} className="rounded-xl border border-border bg-card p-5 sm:p-6">
        <div className="flex items-center gap-2"><ContactRound size={15} className="text-blue-300"/><h2 className="text-sm font-medium">Contact snapshot</h2></div>
        <p className="mt-2 text-[11px] leading-5 text-slate-500">Contact-added timestamps are not currently persisted. Trust state below is current, not a fabricated activity history.</p>
        <div className="mt-4 space-y-2">
          {contactState !== 'ready' && <p className="py-5 text-xs text-slate-500">{contactState === 'loading' ? 'Loading contacts…' : 'Contact list unavailable.'}</p>}
          {(contactState === 'ready' ? contacts : []).slice(0, 5).map(contact => <div key={contact.name} className="flex items-center gap-3 rounded-lg border border-border bg-black/15 p-3">
            <ContactRound size={14} className={contact.trust === 'verified' ? 'text-emerald-300' : contact.trust === 'key_changed' ? 'text-red-300' : 'text-slate-500'}/>
            <p className="min-w-0 flex-1 truncate text-xs text-slate-200">{contact.name}</p>
            <span className="font-mono text-[10px] uppercase text-slate-500">{contact.trust.replace('_', ' ')}</span>
          </div>)}
          {contactState === 'ready' && contacts.length === 0 && <p className="py-5 text-xs text-slate-500">No contacts imported yet.</p>}
        </div>
      </motion.section>
    </motion.div>
  </motion.div>;
}
