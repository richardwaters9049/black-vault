'use client';

import { useEffect, useRef, useState } from 'react';
import { api } from '@/lib/api';
import { decryptFile, decryptFilename, encryptFile, MAX_FILE_SIZE, type FileRecord } from '@/lib/file-crypto';
import { VaultSession } from '@/lib/vault-session';
import { Button } from '@/components/ui/button';

type NamedFile = FileRecord & { name: string; readable: boolean };
export function VaultFiles({ token, session }: { token: string; session: VaultSession }) {
  const [files, setFiles] = useState<NamedFile[]>([]);
  const [error, setError] = useState('');
  const [busy, setBusy] = useState(true);
  const active = useRef(false);

  async function refresh() {
    const result = await api.listFiles(token);
    if (!active.current || !session.unlocked) return;
    setFiles(result.files.map(file => {
      try { return { ...file, name: session.withKey(key => decryptFilename(file, key)), readable: true }; }
      catch { return { ...file, name: 'Unreadable or legacy file', readable: false }; }
    }));
  }
  useEffect(() => {
    active.current = true;
    refresh().catch(() => { if (active.current) setError('Unable to load files'); })
      .finally(() => { if (active.current) setBusy(false); });
    return () => { active.current = false; };
    // This component is mounted only for the current unlocked vault session.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [token, session]);

  async function upload(file: File) {
    if (file.size > MAX_FILE_SIZE) { setError('Files must be 5 MiB or smaller'); return; }
    setBusy(true); setError('');
    let data: Uint8Array | undefined;
    try {
      data = new Uint8Array(await file.arrayBuffer());
      if (!active.current || !session.unlocked) return;
      const encrypted = session.withKey(key => encryptFile(data!, file.name, key));
      data.fill(0);
      await api.uploadFile(token, encrypted);
      await refresh();
    } catch {
      if (active.current) setError('Upload could not be confirmed. Refresh the list before retrying.');
    } finally { data?.fill(0); if (active.current) setBusy(false); }
  }

  async function download(file: NamedFile) {
    setBusy(true); setError('');
    let plaintext: Uint8Array | undefined;
    try {
      const ciphertext = await api.downloadFile(token, file.id);
      if (!active.current || !session.unlocked) return;
      plaintext = session.withKey(key => decryptFile(file, ciphertext, key));
      const blob = new Blob([new Uint8Array(plaintext)], { type: 'application/octet-stream' });
      const url = URL.createObjectURL(blob);
      const anchor = document.createElement('a');
      anchor.href = url;
      anchor.download = file.name.replace(/[\\/\x00-\x1f]/g, '_');
      document.body.appendChild(anchor);
      anchor.click();
      anchor.remove();
      setTimeout(() => URL.revokeObjectURL(url), 1000);
    } catch { if (active.current) setError('Unable to decrypt file. Integrity verification failed or the connection was interrupted.'); }
    finally { plaintext?.fill(0); if (active.current) setBusy(false); }
  }

  return <section className="space-y-4 rounded-xl border border-slate-800 bg-slate-900/50 p-6">
    <h3 className="text-lg font-semibold text-white">Secure files</h3>
    <p className="text-sm text-slate-400">Files and filenames are encrypted in your browser. Maximum 5 MiB per file.</p>
    <label className="block text-sm text-slate-200">Upload a file
      <input className="mt-2 block w-full text-sm text-slate-300" type="file" disabled={busy}
        onChange={event => { const file = event.target.files?.[0]; event.target.value = ''; if (file) void upload(file); }} />
    </label>
    <Button variant="outline" disabled={busy} onClick={async () => {
      setBusy(true); setError('');
      try { await refresh(); } catch { if (active.current) setError('Unable to load files'); }
      finally { if (active.current) setBusy(false); }
    }}>Refresh files</Button>
    {error && <p role="alert" className="text-sm text-red-400">{error}</p>}
    {!busy && files.length === 0 && <p className="text-sm text-slate-400">No encrypted files yet.</p>}
    <ul className="space-y-3">{files.map(file => <li key={file.id} className="flex flex-wrap items-center justify-between gap-3 border-t border-slate-800 pt-3">
      <span className="break-all text-sm text-slate-200">{file.name} <span className="text-slate-500">({file.file_size} bytes)</span></span>
      <Button disabled={busy || !file.readable} onClick={() => void download(file)}>Download</Button>
    </li>)}</ul>
  </section>;
}
