import { useEffect, useState } from 'react';
import {
  AlertTriangle,
  Ban,
  CheckCircle2,
  Clipboard,
  ContactRound,
  Fingerprint,
  KeyRound,
  ShieldCheck,
  ShieldQuestion,
  UserPlus,
} from 'lucide-react';
import { useOperationScope } from '@/lib/useOperationScope';
import { Button } from '@/components/ui/button';
import {
  errorMessage,
  native,
  type ContactTrustState,
  type PublicIdentity,
  type TrustedContact,
} from '@/lib/native';

const trustPresentation: Record<ContactTrustState, { label: string; className: string; icon: typeof ShieldCheck }> = {
  unverified: { label: 'Unverified', className: 'border-amber-400/20 bg-amber-400/10 text-amber-200', icon: ShieldQuestion },
  verified: { label: 'Verified', className: 'border-emerald-400/20 bg-emerald-400/10 text-emerald-200', icon: ShieldCheck },
  key_changed: { label: 'Key changed', className: 'border-red-400/25 bg-red-400/10 text-red-200', icon: AlertTriangle },
  revoked: { label: 'Revoked', className: 'border-slate-500/25 bg-slate-500/10 text-slate-300', icon: Ban },
};

export default function TrustedContacts() {
  const [identity, setIdentity] = useState<PublicIdentity | null>(null);
  const [contacts, setContacts] = useState<TrustedContact[]>([]);
  const [name, setName] = useState('');
  const [document, setDocument] = useState('');
  const [verification, setVerification] = useState<Record<string, string>>({});
  const [busyAction, setBusyAction] = useState('');
  const [error, setError] = useState('');
  const [notice, setNotice] = useState('');
  const [identityFailed, setIdentityFailed] = useState(false);
  const [contactState, setContactState] = useState<'loading' | 'ready' | 'error'>('loading');
  const { start } = useOperationScope();

  useEffect(() => {
    let active = true;
    // Secure identity storage and public contact metadata can fail independently.
    native.getPublicIdentity().then(value => { if (active) setIdentity(value); })
      .catch(reason => { if (active) { setIdentityFailed(true); setError(current => [current, errorMessage(reason)].filter(Boolean).join(' ')); } });
    native.listContacts().then(items => { if (active) { setContacts(items); setContactState('ready'); } })
      .catch(reason => { if (active) { setContactState('error'); setError(current => [current, errorMessage(reason)].filter(Boolean).join(' ')); } });
    return () => { active = false; };
  }, []);

  function replaceContact(updated: TrustedContact) {
    setContacts(current => {
      const without = current.filter(contact => contact.name !== updated.name);
      return [...without, updated].sort((left, right) => left.name.localeCompare(right.name));
    });
  }

  async function importContact(event: React.FormEvent) {
    event.preventDefault();
    if (!name.trim() || !document.trim() || busyAction || contactState !== 'ready') return;
    const isCurrent = start();
    setBusyAction('import'); setError(''); setNotice('');
    try {
      const contact = await native.importContactIdentity(name, document);
      if (!isCurrent()) return;
      replaceContact(contact);
      setName(''); setDocument('');
      setNotice(contact.trust === 'key_changed'
        ? 'Identity changed. Previous verification was removed; verify the new fingerprint out of band.'
        : 'Public identity imported. Verify its fingerprint through a separate trusted channel.');
    } catch (reason) { if (isCurrent()) setError(errorMessage(reason)); }
    finally { if (isCurrent()) setBusyAction(''); }
  }

  async function verifyContact(contact: TrustedContact) {
    const fingerprint = verification[contact.name]?.trim() ?? '';
    if (!fingerprint || busyAction) return;
    const isCurrent = start();
    setBusyAction('verify:' + contact.name); setError(''); setNotice('');
    try {
      const updated = await native.verifyContactFingerprint(contact.name, fingerprint);
      if (!isCurrent()) return;
      replaceContact(updated);
      setVerification(current => ({ ...current, [contact.name]: '' }));
      setNotice(contact.trust === 'key_changed'
        ? `${contact.name}'s new fingerprint is now verified.`
        : `${contact.name}'s fingerprint is now verified.`);
    } catch (reason) { if (isCurrent()) setError(errorMessage(reason)); }
    finally { if (isCurrent()) setBusyAction(''); }
  }

  async function revokeContact(contact: TrustedContact) {
    if (busyAction) return;
    const isCurrent = start();
    setBusyAction('revoke:' + contact.name); setError(''); setNotice('');
    try {
      const updated = await native.revokeContact(contact.name);
      if (!isCurrent()) return;
      replaceContact(updated);
      setVerification(current => ({ ...current, [contact.name]: '' }));
      setNotice(`Trust revoked for ${contact.name}.`);
    } catch (reason) { if (isCurrent()) setError(errorMessage(reason)); }
    finally { if (isCurrent()) setBusyAction(''); }
  }

  async function copyIdentity() {
    if (!identity || busyAction) return;
    setError(''); setNotice('');
    try {
      await navigator.clipboard.writeText(identity.document);
      setNotice('Public identity copied. It contains public keys only.');
    } catch {
      setError('Could not copy the public identity to the clipboard.');
    }
  }

  return <section className="bv-page-enter pt-6">
    <p className="mb-3 font-mono text-[10px] uppercase tracking-[0.2em] text-blue-400">Secure sharing identity</p>
    <div className="flex flex-wrap items-end justify-between gap-4">
      <div>
        <h1 className="text-2xl font-medium tracking-tight sm:text-3xl">Trusted Contacts</h1>
        <p className="mt-2 max-w-2xl text-xs leading-6 text-slate-400">
          Verify fingerprints through a separate trusted channel before treating a contact as verified.
          Display names never establish trust.
        </p>
      </div>
      <div className="rounded-full border border-blue-400/15 bg-blue-500/[0.06] px-3 py-1.5 font-mono text-[10px] uppercase tracking-wider text-blue-200">
        {contacts.filter(contact => contact.trust === 'verified').length} verified
      </div>
    </div>

    {error && <div role="alert" className="mt-5 rounded-lg border border-red-400/20 bg-red-500/[0.07] px-4 py-3 text-xs text-red-200">{error}</div>}
    {notice && <div role="status" className="mt-5 flex items-center gap-2 rounded-lg border border-emerald-400/15 bg-emerald-500/[0.06] px-4 py-3 text-xs text-emerald-200"><CheckCircle2 size={14}/>{notice}</div>}

    <div className="mt-6 grid gap-5 xl:grid-cols-[minmax(0,1.1fr)_minmax(320px,.9fr)]">
      <div className="rounded-xl border border-border bg-card p-5 sm:p-6">
        <div className="flex items-center gap-3">
          <div className="flex size-10 items-center justify-center rounded-lg border border-blue-400/20 bg-blue-500/10 text-blue-300"><Fingerprint size={18}/></div>
          <div>
            <h2 className="text-sm font-medium">Your public identity</h2>
            <p className="mt-0.5 text-[11px] text-slate-500">Share this identity document, then confirm the fingerprint separately.</p>
          </div>
        </div>
        <div className="mt-5 rounded-lg border border-white/[0.06] bg-black/20 p-4">
          <p className="text-[10px] uppercase tracking-wider text-slate-500">Fingerprint</p>
          <p className="mt-2 break-all font-mono text-xs leading-6 text-sky-200">{identity?.fingerprint ?? (identityFailed ? 'Identity unavailable.' : 'Loading identity…')}</p>
        </div>
        <Button variant="outline" className="mt-4" disabled={!identity || Boolean(busyAction)} onClick={() => void copyIdentity()}>
          <Clipboard size={14}/> Copy public identity
        </Button>
        <p className="mt-3 text-[11px] leading-5 text-slate-500">Private encryption and signing keys remain in native secure storage and are never displayed here.</p>
      </div>

      <form onSubmit={importContact} className="rounded-xl border border-border bg-card p-5 sm:p-6">
        <div className="flex items-center gap-3">
          <div className="flex size-10 items-center justify-center rounded-lg border border-blue-400/20 bg-blue-500/10 text-blue-300"><UserPlus size={18}/></div>
          <div><h2 className="text-sm font-medium">Import a contact</h2><p className="mt-0.5 text-[11px] text-slate-500">Public identity document only.</p></div>
        </div>
        <label className="mt-5 block text-[11px] text-slate-400" htmlFor="contact-name">Contact name</label>
        <input id="contact-name" aria-label="Contact name" maxLength={100} value={name} onChange={event => setName(event.target.value)}
          className="mt-2 w-full rounded-lg border border-border bg-black/20 px-3 py-2.5 text-sm text-slate-100 placeholder:text-slate-600" placeholder="e.g. Alice" />
        <label className="mt-4 block text-[11px] text-slate-400" htmlFor="contact-identity">Public identity document</label>
        <textarea id="contact-identity" aria-label="Public identity document" maxLength={4096} rows={6} value={document} onChange={event => setDocument(event.target.value)}
          className="mt-2 w-full resize-none rounded-lg border border-border bg-black/20 px-3 py-2.5 font-mono text-[11px] leading-5 text-slate-300 placeholder:text-slate-600 focus:outline-2 focus:outline-blue-400"
          placeholder="Paste a BLACKVAULT-IDENTITY document" />
        <Button type="submit" className="mt-4" disabled={contactState !== 'ready' || !name.trim() || !document.trim() || Boolean(busyAction)}>
          <UserPlus size={14}/>{busyAction === 'import' ? 'Importing…' : 'Import identity'}
        </Button>
      </form>
    </div>

    <div className="mt-7 flex items-center gap-2">
      <ContactRound size={16} className="text-blue-300"/>
      <h2 className="text-sm font-medium">Saved contacts</h2>
      <span className="ml-1 font-mono text-[10px] text-slate-600">{contacts.length}</span>
    </div>

    {contactState !== 'ready'
      ? <p role="status" className="mt-4 text-xs text-slate-500">{contactState === 'loading' ? 'Loading contacts…' : 'Contact list unavailable.'}</p>
      : contacts.length === 0
      ? <div className="mt-4 rounded-xl border border-dashed border-border bg-white/[0.015] px-6 py-10 text-center">
          <ShieldQuestion size={24} className="mx-auto text-slate-600"/>
          <p className="mt-3 text-sm text-slate-400">No contacts imported yet.</p>
          <p className="mt-1 text-[11px] text-slate-600">Imported identities begin as unverified.</p>
        </div>
      : <div className="mt-4 space-y-4">{contacts.map(contact => {
          const presentation = trustPresentation[contact.trust];
          const StatusIcon = presentation.icon;
          const verifying = busyAction === 'verify:' + contact.name;
          const revoking = busyAction === 'revoke:' + contact.name;
          return <article key={contact.name} className="bv-premium-enter rounded-xl border border-border bg-card p-5 sm:p-6">
            <div className="flex flex-wrap items-start justify-between gap-4">
              <div className="min-w-0">
                <div className="flex items-center gap-2"><KeyRound size={15} className="text-blue-300"/><h3 className="truncate text-sm font-medium">{contact.name}</h3></div>
                <p className="mt-3 text-[10px] uppercase tracking-wider text-slate-500">Current fingerprint</p>
                <p className="mt-1 break-all font-mono text-[11px] leading-5 text-slate-300">{contact.fingerprint}</p>
              </div>
              <span className={'flex items-center gap-1.5 rounded-full border px-2.5 py-1 text-[10px] font-medium uppercase tracking-wider ' + presentation.className}>
                <StatusIcon size={12}/>{presentation.label}
              </span>
            </div>

            {contact.trust === 'key_changed' && <div className="mt-4 rounded-lg border border-red-400/20 bg-red-500/[0.06] p-4">
              <p className="flex items-center gap-2 text-xs font-medium text-red-200"><AlertTriangle size={14}/>Previously verified identity changed</p>
              {contact.previousVerifiedFingerprint && <>
                <p className="mt-3 text-[10px] uppercase tracking-wider text-red-200/60">Previously verified fingerprint</p>
                <p className="mt-1 break-all font-mono text-[11px] leading-5 text-red-100/80">{contact.previousVerifiedFingerprint}</p>
              </>}
              <p className="mt-2 text-[11px] leading-5 text-red-100/60">Do not trust the replacement key until you confirm the current fingerprint with the contact through another trusted channel.</p>
            </div>}

            {contact.trust !== 'revoked' && <div className="mt-5 border-t border-border pt-4">
              <label htmlFor={'verify-' + contact.name} className="text-[11px] text-slate-400">Enter the complete fingerprint you verified independently</label>
              <div className="mt-2 flex flex-col gap-2 lg:flex-row">
                <input id={'verify-' + contact.name} aria-label={'Verify fingerprint for ' + contact.name} value={verification[contact.name] ?? ''}
                  onChange={event => setVerification(current => ({ ...current, [contact.name]: event.target.value }))}
                  className="min-w-0 flex-1 rounded-lg border border-border bg-black/20 px-3 py-2.5 font-mono text-[11px] text-slate-200 placeholder:text-slate-600"
                  placeholder="XXXXXXXX XXXXXXXX XXXXXXXX XXXXXXXX …" />
                <Button variant="outline" disabled={!verification[contact.name]?.trim() || Boolean(busyAction)} onClick={() => void verifyContact(contact)}>
                  <ShieldCheck size={14}/>{verifying ? 'Verifying…' : contact.trust === 'verified' ? 'Re-verify' : 'Verify fingerprint'}
                </Button>
              </div>
            </div>}

            <div className="mt-4 flex items-center justify-between gap-3">
              <p className="text-[10px] leading-4 text-slate-600">{contact.trust === 'revoked' ? 'This contact remains stored but is not trusted.' : 'Verification compares against the current cryptographic identity.'}</p>
              {contact.trust !== 'revoked' && <Button variant="outline" disabled={Boolean(busyAction)} onClick={() => void revokeContact(contact)}
                className="shrink-0 border-red-400/15 text-red-200 hover:bg-red-500/10 hover:text-red-100">
                <Ban size={13}/>{revoking ? 'Revoking…' : 'Revoke trust'}
              </Button>}
            </div>
          </article>;
        })}</div>}
  </section>;
}
