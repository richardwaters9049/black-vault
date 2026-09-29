import { useEffect, useState, type FormEvent } from 'react';
import { CheckCircle2, FilePlus2, FolderOpen, Save, ShieldCheck } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { errorMessage, formatFileSize, native, type VaultFile } from '@/lib/native';

const TYPES = ['txt', 'md', 'json', 'csv', 'log'] as const;

export default function VaultFileEditor({ vaultId, onFilesChange }: {
  vaultId: string;
  onFilesChange: (files: VaultFile[]) => void;
}) {
  const [files, setFiles] = useState<VaultFile[]>([]);
  const [name, setName] = useState('');
  const [type, setType] = useState<(typeof TYPES)[number]>('txt');
  const [content, setContent] = useState('');
  const [viewing, setViewing] = useState<VaultFile | null>(null);
  const [confirmDelete, setConfirmDelete] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [phase, setPhase] = useState<'idle' | 'working' | 'success' | 'error'>('idle');
  const [status, setStatus] = useState('');

  useEffect(() => {
    let active = true;
    setName(''); setContent(''); setViewing(null); setPhase('idle'); setStatus('');
    native.listVaultFiles().then(items => { if (active) { setFiles(items); onFilesChange(items); } })
      .catch(reason => { if (active) { setPhase('error'); setStatus(errorMessage(reason)); } });
    return () => { active = false; };
  }, [vaultId, onFilesChange]);

  async function save(event: FormEvent) {
    event.preventDefault();
    if (busy) return;
    const filename = viewing ? name.trim() + '.' + type : name.trim() + '.' + type;
    if (!name.trim() || /[\\/\x00-\x1f]/.test(name) || name === '.' || name === '..') {
      setPhase('error'); setStatus('Enter a valid filename without slashes or control characters.'); return;
    }
    if (new TextEncoder().encode(content).byteLength > 5 * 1024 * 1024) {
      setPhase('error'); setStatus('Maximum file size is 5 MiB.'); return;
    }
    setBusy(true); setPhase('working'); setStatus('Scanning and encrypting directly into the vault…');
    try {
      const created = viewing ? await native.updateVaultTextFile(viewing.id, filename, content) : await native.createVaultTextFile(filename, content);
      const next = await native.listVaultFiles();
      setFiles(next); onFilesChange(next);
      setContent(''); setName(''); setViewing(null); setConfirmDelete(null);
      setPhase('success'); setStatus(created.name + ' encrypted and saved in the vault. No plaintext copy was created.');
    } catch (reason) { setPhase('error'); setStatus(errorMessage(reason)); }
    finally { setBusy(false); }
  }

  async function open(file: VaultFile) {
    if (busy) return;
    setBusy(true); setPhase('working'); setStatus('Authenticating and opening encrypted file…');
    try {
      const value = await native.readVaultTextFile(file.id);
      setViewing(file); setContent(value);
      const extension = TYPES.find(ext => file.name.toLowerCase().endsWith('.' + ext)) ?? 'txt';
      setType(extension); setName(file.name.slice(0, -(extension.length + 1))); setConfirmDelete(null);
      setPhase('success'); setStatus('Opened ' + file.name + ' in the session editor. Close it to clear the editor.');
    } catch (reason) { setPhase('error'); setStatus(errorMessage(reason)); }
    finally { setBusy(false); }
  }
  function close() { setViewing(null); setName(''); setContent(''); setConfirmDelete(null); setPhase('idle'); setStatus('Editor cleared.'); }
  async function remove(file: VaultFile) {
    if (busy) return;
    if (confirmDelete !== file.id) { setConfirmDelete(file.id); return; }
    setBusy(true); setPhase('working'); setStatus('Deleting encrypted file…');
    try {
      await native.deleteVaultFile(file.id);
      const next = await native.listVaultFiles();
      setFiles(next); onFilesChange(next);
      if (viewing?.id === file.id) close();
      setConfirmDelete(null); setPhase('success'); setStatus('Encrypted file deleted. This cannot be undone.');
    } catch (reason) { setPhase('error'); setStatus(errorMessage(reason)); }
    finally { setBusy(false); }
  }

  return <section aria-label="Create files in vault" className="mt-6 grid gap-5 lg:grid-cols-[minmax(0,1.5fr)_minmax(210px,1fr)]">
    <form onSubmit={event => void save(event)} className="rounded-xl border border-border bg-card p-5 sm:p-6">
      <div className="flex items-center gap-2"><FilePlus2 size={17} className="text-sky-300"/><h2 className="text-sm font-medium">{viewing ? 'Edit encrypted document' : 'New encrypted document'}</h2></div>
      <p className="mt-2 text-xs leading-5 text-slate-400">Text files are encrypted by Rust and persisted only as vault envelopes. The editor exists in memory while your vault is unlocked.</p>
      {viewing && <div className="mt-5 flex items-center justify-between gap-3 rounded-lg border border-blue-400/15 bg-blue-400/5 p-3"><span className="min-w-0 break-all text-xs text-sky-200">{viewing.name}</span><Button type="button" variant="outline" disabled={busy} onClick={close}>Close file</Button></div>}
        <div className="mt-5 grid gap-3 sm:grid-cols-[minmax(0,1fr)_110px]">
          <label className="text-xs text-slate-300">Filename (without extension)<input aria-label="Filename" value={name} onChange={event => setName(event.target.value)} maxLength={100} required disabled={busy} placeholder="My secure notes" className="mt-2 w-full rounded-lg border border-border bg-background px-3 py-2.5 text-sm outline-none focus:border-blue-400"/></label>
          <label className="text-xs text-slate-300">File type<select aria-label="File type" value={type} disabled={busy} onChange={event => setType(event.target.value as typeof type)} className="mt-2 w-full rounded-lg border border-border bg-background px-3 py-2.5 text-sm outline-none focus:border-blue-400">{TYPES.map(item => <option key={item} value={item}>.{item}</option>)}</select></label>
        </div>
      <label className="mt-5 block text-xs text-slate-300">Contents<textarea aria-label="File contents" value={content} disabled={busy} onChange={event => { setContent(event.target.value); if (phase !== 'idle') setPhase('idle'); }} rows={14} spellCheck={false} placeholder="Start writing…" className="mt-2 w-full resize-y rounded-lg border border-border bg-[#080f1a] p-4 font-mono text-xs leading-6 text-slate-100 outline-none focus:border-blue-400"/></label>
      <div className="mt-4 flex flex-wrap items-center justify-between gap-3"><span className="font-mono text-[10px] text-slate-500">{formatFileSize(new TextEncoder().encode(content).byteLength)} / 5 MiB</span><Button type="submit" disabled={busy || !name.trim()}><Save size={14}/>{busy ? 'Encrypting…' : viewing ? 'Encrypt & save changes' : 'Encrypt & save to vault'}</Button></div>
      {phase === 'working' && <div role="progressbar" aria-label="Vault file operation" className="mt-4 h-1 overflow-hidden rounded-full bg-blue-400/10"><div className="bv-operation-progress h-full w-1/3 rounded-full bg-sky-400"/></div>}
      <p role={phase === 'error' ? 'alert' : 'status'} aria-live="polite" className={'mt-4 flex items-start gap-2 text-xs leading-5 ' + (phase === 'error' ? 'text-red-300' : phase === 'success' ? 'bv-operation-confirm text-emerald-300' : 'text-slate-400')}>{phase === 'success' && <CheckCircle2 size={15} className="shrink-0"/>}{status}</p>
    </form>
    <aside className="rounded-xl border border-border bg-card p-5 sm:p-6"><div className="flex items-center gap-2"><FolderOpen size={16} className="text-sky-300"/><h2 className="text-sm font-medium">Vault documents</h2><span className="ml-auto font-mono text-[10px] text-slate-500">{files.length}</span></div>
      <p className="mt-2 text-xs leading-5 text-slate-500">Open supported text documents without exporting them. Viewing does not write plaintext to disk.</p>
      <div className="mt-4 space-y-2">{files.filter(file => TYPES.some(extension => file.name.toLowerCase().endsWith('.' + extension))).map(file =>
        <div key={file.id} className="flex items-center gap-2 rounded-lg border border-border bg-[#0b111b] p-2 text-xs">
          <button type="button" disabled={busy} onClick={() => void open(file)} className="flex min-w-0 flex-1 items-center gap-2 p-1 text-left transition-colors hover:text-sky-300 disabled:opacity-50" aria-label={'Edit ' + file.name}>
            <ShieldCheck size={14} className="shrink-0 text-sky-300"/><span className="min-w-0 flex-1 break-all">{file.name}</span><span className="shrink-0 font-mono text-[10px] text-slate-500">{formatFileSize(file.sizeBytes)}</span>
          </button>
          <button type="button" disabled={busy} onClick={() => void remove(file)} className={'rounded px-2 py-1 transition-colors ' + (confirmDelete === file.id ? 'bg-red-500/20 text-red-200' : 'text-slate-500 hover:text-red-300')} aria-label={(confirmDelete === file.id ? 'Confirm delete ' : 'Delete ') + file.name}>{confirmDelete === file.id ? 'Confirm' : 'Delete'}</button>
          {confirmDelete === file.id && <button type="button" onClick={() => setConfirmDelete(null)} className="text-slate-400">Cancel</button>}
        </div>)}</div>
      {files.length === 0 && <p className="mt-5 text-xs text-slate-500">Create your first file to see it here.</p>}
    </aside>
  </section>;
}
