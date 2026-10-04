import { useEffect, useState, type FormEvent } from 'react';
import { motion, useReducedMotion, type Variants } from 'framer-motion';
import { ArrowRight, Eye, EyeOff, Fingerprint, KeyRound, Lock, Plus, ShieldCheck, Trash2 } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { errorMessage, native, type VaultSummary } from '@/lib/native';

const accessContainer: Variants = {
  hidden: {},
  show: { transition: { staggerChildren: 0.07, delayChildren: 0.05 } },
};

const accessItem: Variants = {
  hidden: { opacity: 0, y: 12, scale: 0.997 },
  show: { opacity: 1, y: 0, scale: 1, transition: { duration: 0.32, ease: [0.22, 1, 0.36, 1] } },
};

export default function VaultAccess({ onAuthenticated }: { onAuthenticated: (vault: VaultSummary) => void }) {
  const [vaults, setVaults] = useState<VaultSummary[]>([]);
  const [selected, setSelected] = useState('');
  const [creating, setCreating] = useState(false);
  const [name, setName] = useState('');
  const [password, setPassword] = useState(import.meta.env.DEV ? (import.meta.env.VITE_DEV_VAULT_PASSWORD ?? '') : '');
  const [showPassword, setShowPassword] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const [touchIdAvailable, setTouchIdAvailable] = useState(false);
  const [biometricMessage, setBiometricMessage] = useState('');
  const [deleting, setDeleting] = useState(false);
  const [confirmation, setConfirmation] = useState('');
  const reduceMotion = useReducedMotion();
  const confirmationMatches = selected !== '' && confirmation === `DELETE ${selected}`;

  useEffect(() => {
    let active = true;
    Promise.all([native.listVaults(), native.getBiometricStatus()]).then(([items, biometric]) => {
      if (!active) return;
      setVaults(items);
      setSelected(items[0]?.id ?? '');
      setCreating(items.length === 0);
      setTouchIdAvailable(biometric.available && biometric.kind === 'touch_id');
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

  async function deleteVault() {
    if (busy || !confirmationMatches) return;
    setBusy(true); setError(''); setBiometricMessage('');
    try {
      if (touchIdAvailable) {
        const result = await native.authenticateBiometric();
        if (!result.authenticated) return;
      }
      await native.deleteLockedVault(selected, confirmation);
      const remaining = vaults.filter(vault => vault.id !== selected);
      setVaults(remaining);
      setSelected(remaining[0]?.id ?? '');
      setDeleting(false);
      setConfirmation('');
      setCreating(remaining.length === 0);
      setBiometricMessage('Vault permanently deleted from this device.');
    } catch (reason) {
      setError(errorMessage(reason));
    } finally {
      setBusy(false);
    }
  }

  async function verifyTouchId() {
    if (busy) return;
    setBusy(true); setError(''); setBiometricMessage('');
    try {
      const result = await native.authenticateBiometric();
      if (result.authenticated) {
        setBiometricMessage('Touch ID verified. The vault remains locked until biometric vault access is enrolled.');
      }
    } catch (reason) {
      setError(errorMessage(reason));
    } finally {
      setBusy(false);
    }
  }

  return <motion.section
    aria-labelledby="vault-access-title"
    className="mt-6 overflow-hidden rounded-xl border border-blue-400/15 bg-card"
    variants={accessContainer}
    initial={reduceMotion ? false : "hidden"}
    animate="show"
  >
    <motion.div variants={accessItem} className="border-b border-border bg-blue-500/[0.035] px-5 py-5 sm:px-6">
      <div className="flex items-center gap-3">
        <div className="flex size-10 items-center justify-center rounded-lg border border-blue-400/20 bg-blue-500/10 text-blue-300"><Lock size={17}/></div>
        <div>
          <h2 id="vault-access-title" className="text-sm font-medium">{creating ? 'Create a protected vault' : 'Authenticate to open a vault'}</h2>
          <p className="mt-1 text-[11px] leading-5 text-slate-500">Protected filenames and contents remain unavailable until native vault authentication succeeds.</p>
        </div>
      </div>
    </motion.div>

    <motion.form variants={accessContainer} onSubmit={event => void submit(event)} className="p-5 sm:p-6">
      {!creating && <motion.label variants={accessItem} className="block text-xs text-slate-400">Locked vault
        <select aria-label="Locked vault" value={selected} onChange={event => setSelected(event.target.value)} disabled={busy}
          className="mt-2 w-full rounded-lg border border-border bg-background px-3 py-3 text-sm text-slate-100 outline-none focus:border-sky-400">
          {vaults.map(vault => <option key={vault.id} value={vault.id}>Vault {vault.id.slice(0, 8)}</option>)}
        </select>
      </motion.label>}

      {creating && <motion.label variants={accessItem} className="block text-xs text-slate-400">Vault name
        <input aria-label="Vault name" value={name} onChange={event => setName(event.target.value)} maxLength={80} required disabled={busy}
          autoComplete="off" placeholder="Personal" className="mt-2 w-full rounded-lg border border-border bg-background px-3 py-3 text-sm text-slate-100 outline-none focus:border-sky-400"/>
      </motion.label>}

      <motion.label variants={accessItem} className="mt-4 block text-xs text-slate-400">Vault password
        <div className="relative mt-2">
          <input aria-label="Vault password" type={showPassword ? 'text' : 'password'} value={password} onChange={event => setPassword(event.target.value)}
            minLength={creating ? 12 : undefined} required disabled={busy} autoComplete={creating ? 'new-password' : 'current-password'}
            className="w-full rounded-lg border border-border bg-background px-3 py-3 pr-20 text-sm text-slate-100 outline-none focus:border-sky-400"/>
          <button type="button" aria-label={showPassword ? 'Hide vault password' : 'Show vault password'} onClick={() => setShowPassword(value => !value)}
            className="absolute inset-y-0 right-2 flex items-center gap-1.5 px-2 text-xs text-sky-400">
            {showPassword ? <EyeOff size={14}/> : <Eye size={14}/>} {showPassword ? 'Hide' : 'Show'}
          </button>
        </div>
      </motion.label>

      {error && <p role="alert" className="mt-4 rounded-lg border border-red-400/20 bg-red-500/[0.06] p-3 text-xs text-red-200">{error}</p>}
      {biometricMessage && <p role="status" className="mt-4 rounded-lg border border-emerald-400/20 bg-emerald-500/[0.06] p-3 text-xs text-emerald-200">{biometricMessage}</p>}

      {!creating && selected && <div className="mt-5 border-t border-border pt-4">
        <button type="button" disabled={busy} onClick={() => { setDeleting(value => !value); setConfirmation(''); setError(''); }}
          className="text-xs text-red-300 hover:text-red-200">{deleting ? 'Cancel deletion' : 'Delete this vault'}</button>
        {deleting && <div className="mt-3 space-y-3 rounded-lg border border-red-400/25 bg-red-500/5 p-4">
          <p className="text-xs leading-5 text-red-200">Permanent deletion removes this local encrypted vault and its protected files. Backups or copies elsewhere are unaffected, and this is not secure SSD erasure.</p>
          <label htmlFor="delete-vault-confirmation" className="block break-all font-mono text-[11px] text-slate-400">Type DELETE {selected} to confirm.</label>
          <input id="delete-vault-confirmation" type="text" aria-label="Delete vault confirmation" placeholder={`DELETE ${selected}`}
            spellCheck={false} autoCapitalize="off" autoCorrect="off" autoComplete="off" value={confirmation}
            onChange={event => setConfirmation(event.target.value)} disabled={busy}
            className="w-full rounded-lg border border-red-400/30 bg-background p-3 font-mono text-xs outline-none"/>
          <p role="status" className={`text-[11px] ${confirmationMatches ? 'text-emerald-300' : 'text-slate-400'}`}>
            {confirmationMatches
              ? touchIdAvailable ? 'Confirmation matched. Touch ID will be required next.' : 'Confirmation matched. You can now delete this vault.'
              : 'The delete button unlocks only when the full confirmation matches.'}
          </p>
          <Button type="button" variant="outline" disabled={busy || !confirmationMatches} onClick={() => void deleteVault()}>
            <Trash2 size={14}/> {touchIdAvailable ? 'Authenticate & permanently delete' : 'Permanently delete vault'}
          </Button>
        </div>}
      </div>}

      <motion.div variants={accessItem} className="mt-5 flex flex-wrap gap-2">
        <Button type="submit" disabled={busy || !password || (creating ? !name.trim() : !selected)}>
          {creating ? <Plus size={14}/> : <KeyRound size={14}/>}
          {busy ? 'Authenticating…' : creating ? 'Create & open vault' : 'Authenticate & open'}
          {!busy && <ArrowRight size={14}/>}
        </Button>
        {vaults.length > 0 && <Button type="button" variant="outline" disabled={busy} onClick={() => { setCreating(value => !value); setPassword(''); setError(''); }}>
          {creating ? 'Choose existing vault' : 'Create another vault'}
        </Button>}
        {touchIdAvailable && <Button type="button" variant="outline" disabled={busy} onClick={() => void verifyTouchId()}>
          <Fingerprint size={14}/> Verify Touch ID
        </Button>}
      </motion.div>
    </motion.form>

    <motion.div variants={accessItem} className="flex items-start gap-2 border-t border-border bg-black/15 px-5 py-4 text-[11px] leading-5 text-slate-500 sm:px-6">
      <ShieldCheck size={14} className="mt-0.5 shrink-0 text-blue-300"/>
      Password authentication remains the cryptographic fallback. Touch ID verification uses macOS Local Authentication; Black Vault never receives or stores fingerprint data.
    </motion.div>
  </motion.section>;
}
