import { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { native, type VaultFile } from '@/lib/native';
import VaultFileEditor from './VaultFileEditor';

const file: VaultFile = { id: 'file-id', name: 'notes.txt', sizeBytes: 7 };
let root: Root;
let host: HTMLDivElement;
const onFilesChange = vi.fn();

beforeEach(async () => {
  vi.spyOn(native, 'listVaultFiles').mockResolvedValue([file]);
  onFilesChange.mockClear();
  host = document.createElement('div'); document.body.appendChild(host); root = createRoot(host);
  await act(async () => root.render(<VaultFileEditor vaultId="first" files={[file]} onFilesChange={onFilesChange}/>));
});
afterEach(async () => { await act(async () => root.unmount()); host.remove(); vi.restoreAllMocks(); });
async function input(label: string, value: string) {
  const element = host.querySelector<HTMLInputElement | HTMLTextAreaElement>(`[aria-label="${label}"]`)!;
  const prototype = element instanceof HTMLTextAreaElement ? HTMLTextAreaElement.prototype : HTMLInputElement.prototype;
  await act(async () => {
    Object.getOwnPropertyDescriptor(prototype, 'value')!.set!.call(element, value);
    element.dispatchEvent(new Event('input', { bubbles: true }));
  });
}
async function click(label: string) {
  const button = host.querySelector<HTMLButtonElement>(`button[aria-label="${label}"]`)!;
  await act(async () => button.click());
}
it('uses a committed save result without a fallible follow-up read', async () => {
  vi.spyOn(native, 'createVaultTextFile').mockResolvedValue({ ...file, id: 'new-file', name: 'new.txt' });
  vi.mocked(native.listVaultFiles).mockRejectedValue(new Error('refresh unavailable'));
  await input('Filename', 'new'); await input('File contents', 'private draft');
  await act(async () => host.querySelector('form')!.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true })));
  expect(host.textContent).toContain('new.txt encrypted and saved');
  expect(native.listVaultFiles).not.toHaveBeenCalled();
  expect(host.querySelector<HTMLTextAreaElement>('[aria-label="File contents"]')!.value).toBe('');
  expect(onFilesChange).toHaveBeenLastCalledWith([file, { ...file, id: 'new-file', name: 'new.txt' }]);
});
it('does not expose a late plaintext read in a different vault', async () => {
  let resolve!: (value: string) => void;
  vi.spyOn(native, 'readVaultTextFile').mockReturnValue(new Promise(done => { resolve = done; }));
  await click('Edit notes.txt');
  await act(async () => root.render(<VaultFileEditor vaultId="second" files={[file]} onFilesChange={onFilesChange}/>));
  await act(async () => resolve('late private contents'));
  expect(host.querySelector<HTMLTextAreaElement>('[aria-label="File contents"]')!.value).toBe('');
  expect(host.textContent).not.toContain('Opened notes.txt');
});
it('does not publish a late save result after the editor unmounts', async () => {
  let resolve!: (value: VaultFile) => void;
  vi.spyOn(native, 'createVaultTextFile').mockReturnValue(new Promise(done => { resolve = done; }));
  await input('Filename', 'new');
  await act(async () => host.querySelector('form')!.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true })));
  onFilesChange.mockClear();
  await act(async () => root.render(<></>));
  await act(async () => resolve(file));
  expect(onFilesChange).not.toHaveBeenCalled();
});
it('requires confirmation and removes committed deletion without re-reading storage', async () => {
  const remove = vi.spyOn(native, 'deleteVaultFile').mockResolvedValue();
  vi.mocked(native.listVaultFiles).mockRejectedValue(new Error('refresh unavailable'));
  await click('Delete notes.txt'); expect(remove).not.toHaveBeenCalled();
  await click('Confirm delete notes.txt');
  expect(remove).toHaveBeenCalledOnce();
  expect(host.textContent).toContain('Encrypted file deleted');
  expect(onFilesChange).toHaveBeenLastCalledWith([]);
});
