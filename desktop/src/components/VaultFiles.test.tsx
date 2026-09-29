import { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { mockIPC } from '@tauri-apps/api/mocks';
import VaultFiles from './VaultFiles';

let root: Root;
let host: HTMLDivElement;
let calls: string[];
let files: Array<{ id: string; name: string; sizeBytes: number }>;
const id = '0123456789abcdef0123456789abcdef';
beforeEach(async () => {
  vi.stubGlobal('isTauri', true);
  calls = []; files = [];
  mockIPC((command) => {
    calls.push(command);
    if (command === 'list_vault_files') return files;
    if (command === 'import_vault_file') {
      files = [{ id, name: 'secret.txt', sizeBytes: 6 }];
      return files[0];
    }
    if (command === 'export_vault_file') return files[0] ?? null;
    throw new Error('Unexpected command');
  });
  host = document.createElement('div');
  document.body.appendChild(host);
  root = createRoot(host);
  await act(async () => root.render(<VaultFiles vaultId="vault-id" />));
});
afterEach(async () => { await act(async () => root.unmount()); host.remove(); vi.unstubAllGlobals(); });
async function click(label: string) {
  const button = Array.from(host.querySelectorAll('button')).find(item => item.textContent?.includes(label));
  if (!button) throw new Error('Missing button: ' + label);
  await act(async () => { button.click(); });
}
it('imports, lists and exports using dedicated native commands', async () => {
  expect(host.textContent).toContain('No encrypted files');
  await click('Select, scan and encrypt');
  expect(host.textContent).toContain('secret.txt');
  expect(host.textContent).toContain('6 bytes');
  await click('Decryption');
  await click('Decrypt & export');
  expect(host.textContent).toContain('plaintext');
  expect(calls).toEqual(['list_vault_files', 'import_vault_file', 'list_vault_files', 'export_vault_file']);
});
it('does not list plaintext filenames when the vault component unmounts', async () => {
  await click('Select, scan and encrypt');
  await act(async () => root.render(<></>));
  expect(host.textContent).not.toContain('secret.txt');
});
