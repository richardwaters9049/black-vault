import { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { mockIPC } from '@tauri-apps/api/mocks';
import VaultFiles from './VaultFiles';

const eventState = vi.hoisted(() => ({
  callbacks: new Map<string, () => void | Promise<void>>(),
  failListener: '',
}));
vi.mock('@tauri-apps/api/event', () => ({
  listen: async (name: string, callback: () => void | Promise<void>) => {
    if (eventState.failListener === name) throw new Error('listener unavailable');
    eventState.callbacks.set(name, callback);
    return () => { eventState.callbacks.delete(name); };
  },
}));
vi.mock('@tauri-apps/api/webview', () => ({ getCurrentWebview: () => ({ onDragDropEvent: async () => () => {} }) }));

let root: Root;
let host: HTMLDivElement;
let calls: string[];
let files: Array<{ id: string; name: string; sizeBytes: number }>;
const id = '0123456789abcdef0123456789abcdef';
beforeEach(async () => {
  vi.stubGlobal('isTauri', true);
  calls = []; files = []; eventState.callbacks.clear(); eventState.failListener = '';
  mockIPC((command) => {
    calls.push(command);
    if (command === 'list_vault_files') return files;
    if (command === 'describe_dropped_file') return { name: 'finder-file.txt', sizeBytes: 120 };
    if (command === 'import_vault_file') {
      files = [{ id, name: 'secret.txt', sizeBytes: 6 }];
      return files[0];
    }
    if (command === 'decrypt_vault_file') return files[0];
    if (command === 'export_vault_file') return files[0] ?? null;
    if (command === 'import_example_file') { files = [{ id, name: 'example-notes.txt', sizeBytes: 10 }]; return files[0]; }
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
it('imports into the vault and decrypts in memory without exporting plaintext', async () => {
  expect(host.textContent).toContain('No encrypted files');
  await click('Choose file to encrypt');
  expect(host.textContent).toContain('secret.txt');
  expect(host.textContent).toContain('6 bytes');
  await click('Decryption');
  await act(async () => root.render(<VaultFiles vaultId="vault-id" chosenDecrypt={{ id, nonce: 1 }} />));
  const decrypt = Array.from(host.querySelectorAll('button')).find(button => button.textContent?.trim() === 'Decrypt');
  if (!decrypt) throw new Error('Decrypt confirmation missing');
  await act(async () => decrypt.click());
  expect(host.textContent).toContain('No plaintext file was saved');
  expect(host.textContent).toContain('Decrypted vault files');
  expect(calls).toEqual(['list_vault_files', 'import_vault_file', 'list_vault_files', 'decrypt_vault_file']);
  expect(calls).not.toContain('export_vault_file');
});
it('does not list plaintext filenames when the vault component unmounts', async () => {
  await click('Choose file to encrypt');
  await act(async () => root.render(<></>));
  expect(host.textContent).not.toContain('secret.txt');
});

it('stages a dragged example and waits for explicit Encrypt confirmation', async () => {
  const target = host.querySelector('[role="tabpanel"] > div');
  if (!target) throw new Error('Drop target missing');
  const payload = { getData: (type: string) => type === 'text/plain' ? 'black-vault-example:notes' : '', types: ['text/plain'] };
  await act(async () => {
    const event = new Event('drop', { bubbles: true, cancelable: true });
    Object.defineProperty(event, 'dataTransfer', { value: payload });
    target.dispatchEvent(event);
  });
  expect(host.textContent).toContain('example-notes.txt selected');
  expect(calls).not.toContain('import_example_file');
  const confirm = Array.from(host.querySelectorAll('button')).find(item => item.textContent?.trim() === 'Encrypt');
  if (!confirm) throw new Error('Missing Encrypt confirmation');
  await act(async () => confirm.click());
  expect(calls).toContain('import_example_file');
  expect(host.textContent).toContain('encrypted and stored');
});

it('stages an example selected by clicking the sidebar, without relying on native drag events', async () => {
  await act(async () => root.render(<VaultFiles vaultId="vault-id" chosenExample={{ id: 'notes', nonce: 1 }} />));
  expect(host.textContent).toContain('example-notes.txt selected');
  expect(calls).not.toContain('import_example_file');
  const confirm = Array.from(host.querySelectorAll('button')).find(item => item.textContent?.trim() === 'Encrypt');
  if (!confirm) throw new Error('Missing Encrypt confirmation');
  await act(async () => confirm.click());
  expect(calls).toContain('import_example_file');
});

it('stages a native Finder file and replaces the previous selection on a second drop', async () => {
  await act(async () => { await Promise.resolve(); });
  const dropped = eventState.callbacks.get('black-vault-file-staged');
  if (!dropped) throw new Error('Native drop listener missing');
  await act(async () => { await dropped(); });
  expect(host.textContent).toContain('finder-file.txt');
  expect(host.textContent).toContain('File ready for encryption');
  expect(calls).not.toContain('import_dropped_file');
  await act(async () => root.render(<VaultFiles vaultId="vault-id" chosenExample={{ id: 'notes', nonce: 1 }} />));
  expect(host.textContent).toContain('example-notes.txt');
  await act(async () => { await dropped(); });
  expect(host.textContent).toContain('finder-file.txt');
  expect(host.textContent).not.toContain('example-notes.txt selected');
});

it('surfaces native listener registration failures without logging sensitive details', async () => {
  const error = vi.spyOn(console, 'error').mockImplementation(() => {});
  eventState.failListener = 'black-vault-file-drop-multiple';
  await click('Decryption');
  await act(async () => { await Promise.resolve(); });
  expect(host.textContent).toContain('listener unavailable');
  expect(error).toHaveBeenCalledWith('[vault-drop][ui] failed to register multi-drop listener (details withheld)');
  error.mockRestore();
});
