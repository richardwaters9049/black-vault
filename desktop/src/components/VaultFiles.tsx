import { useEffect, useState } from 'react';
import { Download, FileLock2, FolderPlus } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { errorMessage, formatFileSize, native, type VaultFile } from '@/lib/native';

export default function VaultFiles({ vaultId }: { vaultId: string }) {
  const [files, setFiles] = useState<VaultFile[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const [notice, setNotice] = useState('');
  useEffect(() => {
    let mounted = true;
    setFiles([]); setError(''); setNotice('');
    native.listVaultFiles().then(items => { if (mounted) setFiles(items); })
      .catch(reason => { if (mounted) setError(errorMessage(reason)); });
    return () => { mounted = false; };
  }, [vaultId]);

  async function importFile() {
    if (busy) return;
    setBusy(true); setError(''); setNotice('');
    try {
      const imported = await native.importVaultFile();
      if (imported) {
        setFiles(await native.listVaultFiles());
        setNotice('File encrypted and saved inside this vault. The original remains unchanged.');
      }
    } catch (reason) { setError(errorMessage(reason)); }
    finally { setBusy(false); }
  }
  async function exportFile(fileId: string) {
    if (busy) return;
    setBusy(true); setError(''); setNotice('');
    try {
      const exported = await native.exportVaultFile(fileId);
      if (exported) setNotice('Decrypted export saved to your chosen new destination. Treat the exported file as plaintext.');
    } catch (reason) { setError(errorMessage(reason)); }
    finally { setBusy(false); }
  }
  return <section aria-label="Encrypted files" className="mt-6 border-t border-border pt-5">
    <div className="flex flex-wrap items-center justify-between gap-3">
      <div>
        <h4 className="flex items-center gap-2 text-sm font-medium"><FileLock2 size={15} className="text-blue-400" /> Encrypted files</h4>
        <p className="mt-1 text-xs text-slate-400">Authenticated local storage · 5 MiB maximum per file</p>
      </div>
      <Button variant="outline" disabled={busy} onClick={() => void importFile()}><FolderPlus size={15}/>{busy ? 'Working…' : 'Import and encrypt'}</Button>
    </div>
    {files.length === 0 ? <p className="mt-4 text-xs text-slate-500">No encrypted files in this vault.</p>
      : <div className="mt-4 space-y-2">{files.map(file => <div key={file.id} className="flex flex-wrap items-center gap-3 rounded-lg border border-border bg-background p-3">
        <FileLock2 size={17} className="shrink-0 text-blue-300" />
        <div className="min-w-0 flex-1">
          <p className="break-all text-xs font-medium">{file.name}</p>
          <p className="mt-1 font-mono text-[10px] text-slate-500">{formatFileSize(file.sizeBytes)} · {file.id.slice(0, 8)}</p>
        </div>
        <Button variant="outline" disabled={busy} onClick={() => void exportFile(file.id)}><Download size={14}/> Export</Button>
      </div>)}</div>}
    {error && <p role="alert" className="mt-3 text-xs text-red-300">{error}</p>}
    {notice && <p role="status" className="mt-3 text-xs leading-5 text-emerald-300">{notice}</p>}
    <p className="mt-4 text-[11px] leading-5 text-slate-500">Import does not delete the source file. Export creates a new plaintext file and refuses to overwrite an existing destination.</p>
  </section>;
}
