import { useEffect, useState, type DragEvent } from 'react';
import { CheckCircle2, Download, FileLock2, FolderPlus, LockKeyhole, ScanSearch, ShieldCheck, ShieldX } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { EXAMPLE_DRAG_TYPE, examples } from './ExampleFiles';
import { errorMessage, formatFileSize, native, type VaultFile } from '@/lib/native';

const VAULT_FILE_DRAG_TYPE = 'application/x-black-vault-encrypted-file';
type Mode = 'encrypt' | 'decrypt';
type Phase = 'idle' | 'working' | 'success' | 'error';

export default function VaultFiles({ vaultId }: { vaultId: string }) {
  const [files, setFiles] = useState<VaultFile[]>([]);
  const [mode, setMode] = useState<Mode>('encrypt');
  const [busy, setBusy] = useState(false);
  const [over, setOver] = useState(false);
  const [phase, setPhase] = useState<Phase>('idle');
  const [status, setStatus] = useState('');
  useEffect(() => {
    let mounted = true;
    setFiles([]); setPhase('idle'); setStatus(''); setMode('encrypt');
    native.listVaultFiles().then(items => { if (mounted) setFiles(items); })
      .catch(reason => { if (mounted) { setPhase('error'); setStatus(errorMessage(reason)); } });
    return () => { mounted = false; };
  }, [vaultId]);

  async function run(action: () => Promise<string | null>) {
    if (busy) return;
    setBusy(true); setPhase('working'); setStatus(mode === 'encrypt' ? 'Scanning and encrypting locally…' : 'Decrypting and preparing export…');
    try {
      const message = await action();
      if (message) { setPhase('success'); setStatus(message); }
      else { setPhase('idle'); setStatus('Operation cancelled. No changes made.'); }
    } catch (reason) { setPhase('error'); setStatus(errorMessage(reason)); }
    finally { setBusy(false); setOver(false); }
  }
  function importFile() {
    void run(async () => {
      const imported = await native.importVaultFile();
      if (!imported) return null;
      setFiles(await native.listVaultFiles());
      return imported.name + ' scanned, encrypted and saved successfully. This is not antivirus clearance; the source remains unchanged.';
    });
  }
  function importExample(id: string) {
    if (!examples.some(sample => sample.id === id)) return;
    void run(async () => {
      const imported = await native.importExampleFile(id);
      setFiles(await native.listVaultFiles());
      return imported.name + ' scanned, encrypted and saved in your vault.';
    });
  }
  function exportFile(id: string) {
    void run(async () => {
      const exported = await native.exportVaultFile(id);
      return exported ? exported.name + ' decrypted and exported successfully. The exported copy is plaintext.' : null;
    });
  }
  function dragOver(event: DragEvent<HTMLElement>) {
    const type = mode === 'encrypt' ? EXAMPLE_DRAG_TYPE : VAULT_FILE_DRAG_TYPE;
    if (!Array.from(event.dataTransfer.types).includes(type)) return;
    event.preventDefault(); event.dataTransfer.dropEffect = 'copy'; setOver(true);
  }
  function drop(event: DragEvent<HTMLElement>) {
    const type = mode === 'encrypt' ? EXAMPLE_DRAG_TYPE : VAULT_FILE_DRAG_TYPE;
    if (!Array.from(event.dataTransfer.types).includes(type)) return;
    event.preventDefault(); setOver(false);
    if (busy) return;
    const id = event.dataTransfer.getData(type);
    if (mode === 'encrypt') importExample(id);
    else if (files.some(file => file.id === id)) exportFile(id);
  }
  function changeMode(next: Mode) { if (busy) return; setMode(next); setOver(false); setPhase('idle'); setStatus(''); }
  return <section aria-label="File encryption and decryption" className="mt-7 border-t border-border pt-5">
    <div role="tablist" aria-label="File operations" className="grid grid-cols-2 gap-2 rounded-xl border border-blue-400/10 bg-[#080f1a] p-1.5">
      {(['encrypt', 'decrypt'] as const).map(tab => <button key={tab} type="button" role="tab" aria-selected={mode === tab} disabled={busy} onClick={() => changeMode(tab)}
        className={`flex items-center justify-center gap-2 rounded-lg px-3 py-3 text-xs transition-colors ${mode === tab ? 'border border-blue-400/20 bg-blue-500/15 text-sky-200' : 'text-slate-400 hover:text-white'}`}>
        {tab === 'encrypt' ? <FileLock2 size={15} /> : <LockKeyhole size={15} />}{tab === 'encrypt' ? 'Encryption' : 'Decryption'}
      </button>)}
    </div>
    <div role="tabpanel" className="mt-4">
      <div onDragOver={dragOver} onDragLeave={event => { if (!event.currentTarget.contains(event.relatedTarget as Node)) setOver(false); }} onDrop={drop}
        className={`relative rounded-xl border border-dashed p-5 text-center transition-all duration-200 sm:p-7 ${over ? 'scale-[1.01] border-sky-300 bg-blue-500/15 shadow-[0_0_35px_#3b82f620]' : 'border-blue-400/25 bg-[#0b1422]'}`}>
        <div className={`mx-auto grid size-12 place-items-center rounded-xl border border-blue-400/20 bg-blue-400/10 text-blue-300 ${phase === 'working' ? 'bv-operation-working' : ''}`}>
          {phase === 'success' ? <CheckCircle2 size={23} className="text-emerald-300" /> : phase === 'error' ? <ShieldX size={23} className="text-red-300" /> : mode === 'encrypt' ? <ScanSearch size={23} /> : <Download size={23} />}
        </div>
        <h4 className="mt-4 text-sm font-medium">{mode === 'encrypt' ? 'Drag and drop an Example File' : 'Drag and drop an encrypted file'}</h4>
        <p className="mx-auto mt-2 max-w-xs text-[11px] leading-5 text-slate-400">{mode === 'encrypt'
          ? 'Expand Example Files in the sidebar and drag a sample here, or choose a file using the secure macOS picker. Maximum 5 MiB.'
          : 'Drag a file from the encrypted list below into this area, or use its Decrypt & export button. Choose a new destination in the macOS picker.'}</p>
        {mode === 'encrypt' && <Button className="mt-5" variant="outline" disabled={busy} onClick={importFile}><FolderPlus size={14}/>{busy ? 'Processing…' : 'Select, scan and encrypt'}</Button>}
        {phase === 'working' && <div className="mx-auto mt-5 h-1 max-w-xs overflow-hidden rounded-full bg-blue-400/10"><div className="bv-operation-progress h-full w-1/3 rounded-full bg-sky-400" /></div>}
      </div>
      <div aria-live="polite" role={phase === 'error' ? 'alert' : 'status'} className={`mt-3 min-h-6 rounded-lg px-3 py-2 text-xs leading-5 transition-all duration-300 ${phase === 'success' ? 'bv-operation-confirm border border-emerald-400/20 bg-emerald-500/5 text-emerald-300' : phase === 'error' ? 'border border-red-400/20 bg-red-500/5 text-red-300' : 'text-slate-400'}`}>
        {phase === 'success' && <ShieldCheck size={14} className="mr-2 inline" />}{status}
      </div>
      <div className="mt-5 flex items-center justify-between gap-3"><h4 className="flex items-center gap-2 text-xs font-medium"><FileLock2 size={14} className="text-sky-400"/> Encrypted vault files</h4><span className="font-mono text-[10px] text-slate-500">{files.length} stored</span></div>
      {files.length === 0 ? <p className="mt-3 text-xs text-slate-500">No encrypted files yet. Start with an example or choose a file.</p> :
        <div className="mt-3 space-y-2">{files.map(file => <div key={file.id} draggable={!busy && mode === 'decrypt'}
          onDragStart={event => { if (mode !== 'decrypt') { event.preventDefault(); return; } event.dataTransfer.effectAllowed = 'copy'; event.dataTransfer.setData(VAULT_FILE_DRAG_TYPE, file.id); }}
          className={`flex flex-wrap items-center gap-3 rounded-lg border border-border bg-[#0b111b] p-3 ${mode === 'decrypt' ? 'cursor-grab active:cursor-grabbing' : ''}`}>
          <FileLock2 size={16} className="shrink-0 text-blue-300"/><div className="min-w-0 flex-1"><p className="break-all text-xs font-medium">{file.name}</p><p className="mt-1 font-mono text-[10px] text-slate-500">{formatFileSize(file.sizeBytes)} · {file.id.slice(0,8)}</p></div>
          {mode === 'decrypt' && <Button variant="outline" disabled={busy} onClick={() => exportFile(file.id)}><Download size={14}/>Decrypt & export</Button>}
        </div>)}</div>}
      <p className="mt-4 text-[11px] leading-5 text-slate-500">A completed animation confirms the native operation has returned successfully; it is not a simulated security result. Local scanning is a limited static policy check, not comprehensive malware detection. Exported files are plaintext.</p>
    </div>
  </section>;
}
