import { useEffect, useRef, useState, type DragEvent } from 'react';
import { getCurrentWebview } from '@tauri-apps/api/webview';
import { isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { CheckCircle2, Download, FileLock2, FolderPlus, LockKeyhole, ScanSearch, ShieldCheck, ShieldX } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { EXAMPLE_DRAG_TYPE, examples, getActiveExampleDrag, getActiveDecryptDrag } from './ExampleFiles';
import { errorMessage, formatFileSize, native, type VaultFile } from '@/lib/native';

const VAULT_FILE_DRAG_TYPE = 'application/x-black-vault-encrypted-file';
type Mode = 'encrypt' | 'decrypt';
type Phase = 'idle' | 'working' | 'success' | 'error';

export default function VaultFiles({ vaultId, chosenExample, chosenDecrypt, onFilesChange, onDecryptedChange }: {
  vaultId: string;
  chosenExample?: { id: string; nonce: number } | null;
  chosenDecrypt?: { id: string; nonce: number } | null;
  onFilesChange?: (files: VaultFile[]) => void;
  onDecryptedChange?: (files: VaultFile[]) => void;
}) {
  const [files, setFiles] = useState<VaultFile[]>([]);
  const [decryptedFiles, setDecryptedFiles] = useState<VaultFile[]>([]);
  const [mode, setMode] = useState<Mode>('encrypt');
  const [busy, setBusy] = useState(false);
  const [over, setOver] = useState(false);
  const [selection, setSelection] = useState<{ mode: Mode; id: string; name: string } | null>(null);
  const dropZone = useRef<HTMLDivElement>(null);
  const [phase, setPhase] = useState<Phase>('idle');
  const [status, setStatus] = useState('');
  useEffect(() => {
    let mounted = true;
    setFiles([]); setDecryptedFiles([]); onDecryptedChange?.([]); setPhase('idle'); setStatus(''); setMode('encrypt'); setSelection(null);
    native.listVaultFiles().then(items => { if (mounted) { setFiles(items); onFilesChange?.(items); } })
      .catch(reason => { if (mounted) { setPhase('error'); setStatus(errorMessage(reason)); } });
    return () => { mounted = false; };
  }, [vaultId]);

  useEffect(() => {
    if (!chosenExample) return;
    const found = examples.find(item => item.id === chosenExample.id);
    if (!found) return;
    setMode('encrypt');
    setSelection({ mode: 'encrypt', id: found.id, name: found.name });
    setPhase('idle');
    setStatus(found.name + ' selected. Press Encrypt to continue.');
  }, [chosenExample]);

  useEffect(() => {
    if (!chosenDecrypt) return;
    const file = files.find(item => item.id === chosenDecrypt.id);
    if (!file) return;
    setMode('decrypt');
    setSelection({ mode: 'decrypt', id: file.id, name: file.name });
    setPhase('idle');
    setStatus(file.name + ' selected. Press Decrypt to continue.');
  }, [chosenDecrypt, files]);

  async function refreshFiles() {
    const items = await native.listVaultFiles();
    setFiles(items);
    onFilesChange?.(items);
  }

  useEffect(() => {
    if (!isTauri()) { console.info('[vault-drop][ui] Native Tauri runtime not detected'); return; }
    console.info('[vault-drop][ui] registering listeners', { mode, busy });
    let active = true;
    let unlistenDrop: (() => void) | undefined;
    let unlistenMultiple: (() => void) | undefined;
    let unlistenHover: (() => void) | undefined;

    // Rust stages the native Finder path first, then emits this path-free event.
    // This avoids relying on WebKit's inconsistent paths and event ordering.
    listen('black-vault-file-staged', async () => {
      console.info('[vault-drop][ui] native file-staged event received', { active, mode, busy });
      if (!active || mode !== 'encrypt' || busy) { console.warn('[vault-drop][ui] ignored staged event: inactive, wrong mode or busy'); return; }
      setOver(false);
      try {
        console.info('[vault-drop][ui] requesting native metadata');
        const file = await native.describeDroppedFile();
        console.info('[vault-drop][ui] metadata received', { sizeBytes: file.sizeBytes, filenamePresent: Boolean(file.name) });
        if (!active) return;
        setSelection({ mode: 'encrypt', id: 'native-drop', name: file.name });
        console.info('[vault-drop][ui] file staged in encryption area');
        setPhase('idle');
        setStatus(file.name + ' selected (' + formatFileSize(file.sizeBytes) + '). Press Encrypt to continue.');
      } catch (reason) {
        console.error('[vault-drop][ui] metadata request failed', reason);
        if (active) { setPhase('error'); setStatus(errorMessage(reason)); }
      }
    }).then(fn => { console.info('[vault-drop][ui] staged event listener ready'); if (active) unlistenDrop = fn; else fn(); }).catch(reason => {
      console.error('[vault-drop][ui] failed to register staged listener', reason);
      if (active) { setPhase('error'); setStatus(errorMessage(reason)); }
    });
    listen('black-vault-file-drop-multiple', () => {
      console.warn('[vault-drop][ui] native multi/empty drop notification');
      if (active && mode === 'encrypt') {
        setOver(false); setPhase('error');
        setStatus('Drop one Finder file at a time. The previous selection has not been changed.');
      }
    }).then(fn => { if (active) unlistenMultiple = fn; else fn(); }).catch(() => {});
    getCurrentWebview().onDragDropEvent(event => {
      if (event.payload.type !== 'over') console.info('[vault-drop][ui] webview event', { type: event.payload.type, count: 'paths' in event.payload ? event.payload.paths?.length : undefined });
      if (!active || busy) return;
      if (event.payload.type === 'drop') {
        setOver(false);
        if (event.payload.paths.length === 0) {
          const decryptId = getActiveDecryptDrag();
          const encrypted = files.find(item => item.id === decryptId);
          if (encrypted) {
            setMode('decrypt');
            setSelection({ mode: 'decrypt', id: encrypted.id, name: encrypted.name });
            setPhase('idle');
            setStatus(encrypted.name + ' selected. Press Decrypt to continue.');
            console.info('[vault-drop][ui] encrypted sidebar file staged', { id: encrypted.id.slice(0, 8) });
            return;
          }
        }
        // Built-in examples are virtual: Tauri receives an empty paths array
        // and WebKit may suppress the HTML drop callback entirely.
        const id = getActiveExampleDrag();
        if (id && event.payload.paths.length === 0) {
          const sample = examples.find(item => item.id === id);
          if (sample) {
            console.info('[vault-drop][ui] staged internal example from native empty-path drop', { id });
            setMode('encrypt');
            setSelection({ mode: 'encrypt', id, name: sample.name });
            setPhase('idle');
            setStatus(sample.name + ' selected. Press Encrypt to continue.');
          }
        } else if (event.payload.paths.length === 0) {
          console.warn('[vault-drop][ui] empty native drop without active example: check Finder file access / source');
        }
        return;
      }
      if (event.payload.type === 'leave') { setOver(false); return; }
      // Native hover is not a filesystem selection. Never reject an empty
      // paths array: internal Example Files also generate drag events.
      setOver(true);
    }).then(fn => { console.info('[vault-drop][ui] hover listener ready'); if (active) unlistenHover = fn; else fn(); }).catch(reason => { console.error('[vault-drop][ui] hover listener failed', reason); });
    return () => {
      console.info('[vault-drop][ui] listeners cleaned up', { mode, busy });
      active = false;
      unlistenDrop?.(); unlistenMultiple?.(); unlistenHover?.();
    };
  }, [mode, busy, files]);

  async function run(action: () => Promise<string | null>) {
    if (busy) return;
    setBusy(true); setPhase('working'); setStatus(mode === 'encrypt' ? 'Scanning and encrypting into the vault…' : 'Decrypting and verifying in secure memory…');
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
      await refreshFiles();
      return imported.name + ' scanned, encrypted and stored in the vault. The original source remains unchanged.';
    });
  }
  function importExample(id: string) {
    if (!examples.some(sample => sample.id === id)) return;
    void run(async () => {
      const imported = await native.importExampleFile(id);
      await refreshFiles();
      return imported.name + ' scanned, encrypted and stored in your vault.';
    });
  }
  function decryptFile(id: string) {
    void run(async () => {
      const verified = await native.decryptVaultFile(id);
      // Rust verifies authentication in zeroising memory and retains only
      // encrypted bytes on disk. This list is a session-only activity view.
      setDecryptedFiles(previous => {
        const next = [...previous.filter(file => file.id !== verified.id), verified];
        onDecryptedChange?.(next);
        return next;
      });
      return verified.name + ' decrypted and verified in memory. No plaintext file was saved. The encrypted original remains in the vault.';
    });
  }
  function dragOver(event: DragEvent<HTMLElement>) {
    // WebKit/Tauri may omit custom MIME types during dragover; validate IDs at drop.
    event.preventDefault(); event.dataTransfer.dropEffect = 'copy'; setOver(true);
  }
  function drop(event: DragEvent<HTMLElement>) {
    event.preventDefault(); event.stopPropagation(); setOver(false);
    console.info('[vault-drop][ui] HTML drop received', { mode, fileCount: event.dataTransfer.files?.length ?? 0, types: Array.from(event.dataTransfer.types ?? []) });
    if (busy) return;
    const fallback = event.dataTransfer.getData('text/plain');
    const raw = event.dataTransfer.getData(mode === 'encrypt' ? EXAMPLE_DRAG_TYPE : VAULT_FILE_DRAG_TYPE);
    const id = raw || (fallback.startsWith('black-vault-example:') ? fallback.slice('black-vault-example:'.length) : fallback.startsWith('black-vault-file:') ? fallback.slice('black-vault-file:'.length) : '');
    const sample = examples.find(item => item.id === id);
    const encrypted = files.find(item => item.id === id);
    const found = mode === 'encrypt' ? sample ?? encrypted : encrypted ?? sample;
    if (!found) {
      // Finder drops are delivered through Tauri's native drag/drop listener,
      // not through WebKit's HTML DataTransfer payload.
      if (mode === 'encrypt') return;
      setPhase('error'); setStatus('Select an Example File in the sidebar, or drop a Finder file into Encryption.'); return;
    }
    const targetMode: Mode = sample ? 'encrypt' : 'decrypt';
    setMode(targetMode);
    setSelection({ mode: targetMode, id, name: found.name });
    setPhase('idle'); setStatus(found.name + ' selected. Press ' + (targetMode === 'encrypt' ? 'Encrypt' : 'Decrypt') + ' to continue.');
  }
  function changeMode(next: Mode) { if (busy) return; setMode(next); setSelection(null); setOver(false); setPhase('idle'); setStatus(''); }
  function confirmSelection() {
    if (!selection || selection.mode !== mode || busy) return;
    if (mode === 'encrypt' && selection.id === 'native-drop') {
      void run(async () => {
        const imported = await native.importDroppedFile();
        await refreshFiles();
        return imported.name + ' scanned, encrypted and stored in the vault. The source remains unchanged.';
      });
    } else if (mode === 'encrypt') importExample(selection.id);
    else decryptFile(selection.id);
    setSelection(null);
  }
  return <section aria-label="File encryption and decryption" className="mt-7 border-t border-border pt-5">
    <div role="tablist" aria-label="File operations" className="grid grid-cols-2 gap-2 rounded-xl border border-blue-400/10 bg-[#080f1a] p-1.5">
      {(['encrypt', 'decrypt'] as const).map(tab => <button key={tab} type="button" role="tab" aria-selected={mode === tab} disabled={busy} onClick={() => changeMode(tab)}
        className={`flex items-center justify-center gap-2 rounded-lg px-3 py-3 text-xs transition-colors ${mode === tab ? 'border border-blue-400/20 bg-blue-500/15 text-sky-200' : 'text-slate-400 hover:text-white'}`}>
        {tab === 'encrypt' ? <FileLock2 size={15} /> : <LockKeyhole size={15} />}{tab === 'encrypt' ? 'Encryption' : 'Decryption'}
      </button>)}
    </div>
    <div role="tabpanel" className="mt-4">
      <div ref={dropZone} onDragOver={dragOver} onDragLeave={event => { if (!event.currentTarget.contains(event.relatedTarget as Node)) setOver(false); }} onDrop={drop}
        className={`relative rounded-xl border border-dashed p-5 text-center transition-all duration-200 sm:p-7 ${over ? 'scale-[1.01] border-sky-300 bg-blue-500/15 shadow-[0_0_35px_#3b82f620]' : 'border-blue-400/25 bg-[#0b1422]'}`}>
        <div className={`mx-auto grid size-12 place-items-center rounded-xl border border-blue-400/20 bg-blue-400/10 text-blue-300 ${phase === 'working' ? 'bv-operation-working' : ''}`}>
          {phase === 'success' ? <CheckCircle2 size={23} className="text-emerald-300" /> : phase === 'error' ? <ShieldX size={23} className="text-red-300" /> : mode === 'encrypt' ? <ScanSearch size={23} /> : <Download size={23} />}
        </div>
        <h4 className="mt-4 text-sm font-medium">{over ? selection ? 'Drop to replace selected file' : 'Drop to select file' : selection?.mode === mode ? 'File ready for ' + (mode === 'encrypt' ? 'encryption' : 'decryption') : mode === 'encrypt' ? 'Drop a file to encrypt' : 'Drop an encrypted vault file'}</h4>
        <p className="mx-auto mt-2 max-w-xs text-[11px] leading-5 text-slate-400">{mode === 'encrypt'
          ? selection ? 'Drop a different file anywhere over this area to replace the selected file. Nothing is encrypted until you press Encrypt.' : 'Drag a Finder file or an Example File here. Maximum 5 MiB. You can change your selection before encrypting.'
          : selection ? 'Drop another encrypted vault file to replace this selection, then press Decrypt.' : 'Drag a stored file from below, then press Decrypt to choose an export destination.'}</p>
        {selection?.mode === mode && <div className={"mx-auto mt-4 flex max-w-sm items-center justify-between gap-3 rounded-lg border p-3 text-left transition-colors " + (over ? "border-sky-300 bg-sky-400/25" : "border-sky-400/30 bg-sky-400/10")}>
          <div className="flex min-w-0 items-center gap-2"><FileLock2 size={17} className="shrink-0 text-sky-300" /><span className="min-w-0 break-all text-xs text-sky-100">{selection.name}</span></div>
          <button type="button" disabled={busy} onClick={() => { setSelection(null); setStatus('Selection cleared.'); }} className="shrink-0 text-xs text-slate-400 hover:text-white">Remove</button>
        </div>}
        {selection?.mode === mode ? <Button className="mt-4" disabled={busy} onClick={confirmSelection}>{busy ? 'Processing…' : mode === 'encrypt' ? 'Encrypt' : 'Decrypt'}</Button> :
          mode === 'encrypt' && <Button className="mt-5" variant="outline" disabled={busy} onClick={importFile}><FolderPlus size={14}/>{busy ? 'Processing…' : 'Choose file to encrypt'}</Button>}
        {phase === 'working' && <div className="mx-auto mt-5 max-w-xs" role="progressbar" aria-label={mode === 'encrypt' ? 'Encrypting file' : 'Decrypting file'}>
          <div className="h-1 overflow-hidden rounded-full bg-blue-400/10"><div className="bv-operation-progress h-full w-1/3 rounded-full bg-sky-400" /></div>
          <p className="mt-2 text-[11px] text-sky-300">{mode === 'encrypt' ? 'Scanning → Encrypting → Vault' : 'Authenticating → Decrypting → Verified'}</p>
        </div>}
      </div>
      <div aria-live="polite" role={phase === 'error' ? 'alert' : 'status'} className={`mt-3 min-h-6 rounded-lg px-3 py-2 text-xs leading-5 transition-all duration-300 ${phase === 'success' ? 'bv-operation-confirm border border-emerald-400/20 bg-emerald-500/5 text-emerald-300' : phase === 'error' ? 'border border-red-400/20 bg-red-500/5 text-red-300' : 'text-slate-400'}`}>
        {phase === 'success' && <ShieldCheck size={14} className="mr-2 inline" />}{status}
      </div>
      <div className="mt-5 flex items-center justify-between gap-3"><h4 className="flex items-center gap-2 text-xs font-medium"><FileLock2 size={14} className="text-sky-400"/> {mode === 'encrypt' ? 'Encrypted vault files' : 'Decrypted vault files'}</h4><span className="font-mono text-[10px] text-slate-500">{mode === 'encrypt' ? files.length + ' stored' : decryptedFiles.length + ' verified this session'}</span></div>
      {mode === 'encrypt' ? (files.length === 0 ? <p className="mt-3 text-xs text-slate-500">No encrypted files yet. Start with an example or choose a file.</p> :
        <div className="mt-3 space-y-2">{files.map(file => <div key={file.id} className="flex items-center gap-3 rounded-lg border border-border bg-[#0b111b] p-3">
          <FileLock2 size={16} className="shrink-0 text-blue-300"/><div className="min-w-0 flex-1"><p className="break-all text-xs font-medium">{file.name}</p><p className="mt-1 font-mono text-[10px] text-slate-500">{formatFileSize(file.sizeBytes)} · Encrypted in vault</p></div>
        </div>)}</div>) : (decryptedFiles.length === 0 ?
        <p className="mt-3 text-xs leading-5 text-slate-500">No files decrypted this session. Choose a stored encrypted file from the sidebar, then press Decrypt. Plaintext is never saved to disk.</p> :
        <div className="mt-3 space-y-2">{decryptedFiles.map(file => <div key={file.id} className="flex items-center gap-3 rounded-lg border border-emerald-400/15 bg-[#0b111b] p-3">
          <CheckCircle2 size={16} className="shrink-0 text-emerald-300"/><div className="min-w-0 flex-1"><p className="break-all text-xs font-medium">{file.name}</p><p className="mt-1 font-mono text-[10px] text-slate-500">{formatFileSize(file.sizeBytes)} · Verified in memory · No plaintext copy</p></div>
        </div>)}</div>)}
      <p className="mt-4 text-[11px] leading-5 text-slate-500">A completed animation confirms the native operation has returned successfully; it is not a simulated security result. Local scanning is a limited static policy check, not comprehensive malware detection. Decryption verifies contents in zeroising Rust memory; the vault retains only encrypted files.</p>
    </div>
  </section>;
}
