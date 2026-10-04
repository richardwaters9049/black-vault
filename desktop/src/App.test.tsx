import { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { mockIPC } from '@tauri-apps/api/mocks';
import App from './App';
vi.mock('@tauri-apps/api/event', () => ({ listen: async () => () => {} }));
vi.mock('@tauri-apps/api/webview', () => ({ getCurrentWebview: () => ({ onDragDropEvent: async () => () => {} }) }));
import { native, type VaultSummary } from './lib/native';

const id = '1234567890abcdef1234567890abcdef';
let root: Root;
let host: HTMLDivElement;
let vaults: VaultSummary[];
let locks: number;
let invalidPassword: boolean;
let vaultFileReads: number;
let deletedVaults: number;

beforeEach(async () => {
  vi.useFakeTimers();
  vi.stubGlobal('isTauri', true);
  vaults = [];
  locks = 0;
  invalidPassword = false;
  vaultFileReads = 0;
  deletedVaults = 0;
  mockIPC((command, args) => {
    if (command === 'get_desktop_info') return { appName: 'Black Vault', version: '0.1.0', platform: 'macos', architecture: 'aarch64' };
    if (command === 'get_biometric_status') return { available: true, kind: 'touch_id', platform: 'macos' };
    if (command === 'delete_locked_vault') { deletedVaults += 1; vaults = vaults.filter(vault => vault.id !== String((args as { vaultId: string }).vaultId)); return null; }
    if (command === 'lock_vault') {
      locks += 1;
      vaults = vaults.map(vault => ({ ...vault, isLocked: true, name: null }));
      return null;
    }
    if (command === 'list_vaults') return vaults;
    if (command === 'create_vault') {
      const vault = { id, name: String((args as { name: string }).name), isLocked: false };
      vaults = [vault]; return vault;
    }
    if (command === 'unlock_vault') {
      if (invalidPassword) throw { code: 'invalid_credentials', message: 'Incorrect vault password.' };
      const vault = { id, name: 'Personal', isLocked: false };
      vaults = [vault]; return vault;
    }
    if (command === 'select_file') return null;
    if (command === 'list_vault_files') { vaultFileReads += 1; return [{ id: 'secret-file', name: 'private-plan.txt', sizeBytes: 128 }]; }
    if (command === 'list_nested_vaults') return [];
    if (command === 'list_contacts') return [];
    throw new Error('Unexpected command: ' + command);
  });
  host = document.createElement('div');
  document.body.appendChild(host);
  root = createRoot(host);
  await act(async () => { root.render(<App />); });
});
afterEach(async () => { await act(async () => root.unmount()); host.remove(); vi.useRealTimers(); vi.unstubAllGlobals(); vi.unstubAllEnvs(); vi.restoreAllMocks(); });

async function ready() {
  await act(async () => { await vi.advanceTimersByTimeAsync(2800); });
}
async function input(label: string, value: string) {
  const element = host.querySelector<HTMLInputElement>(`input[aria-label="${label}"]`);
  if (!element) throw new Error('Missing input: ' + label);
  const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value')?.set;
  await act(async () => { setter?.call(element, value); element.dispatchEvent(new Event('input', { bubbles: true })); });
}
async function submit() {
  const form = host.querySelector('form');
  if (!form) throw new Error('Missing authentication form');
  await act(async () => { form.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true })); });
}
async function click(label: string) {
  const button = Array.from(host.querySelectorAll('button')).find(element => element.textContent?.includes(label));
  if (!button) throw new Error('Missing button: ' + label);
  await act(async () => { button.click(); });
}

it('shows a startup animation, locks previous state and requires first-time setup', async () => {
  expect(host.getAttribute('aria-label')).not.toBe('Dashboard');
  expect(host.querySelector('[aria-label="Initialising Black Vault"]')).not.toBeNull();
  await ready();
  expect(locks).toBe(1);
  expect(host.textContent).toContain('Overview');
  expect(host.textContent).toContain('Vault contents hidden while locked');
  expect(host.textContent).not.toContain('private-plan.txt');
  expect(vaultFileReads).toBe(0);
  expect(host.textContent).not.toContain('Authenticated vault workspace');
});

it('creates a vault, enters the dashboard and locks it on sign-out', async () => {
  await ready();
  await click('Decryption');
  await input('Vault name', 'Personal');
  await input('Vault password', 'correct horse battery staple');
  await submit();
  expect(host.textContent).toContain('Authenticated vault workspace');
  expect(host.textContent).toContain('Open, examine and edit protected documents');
  await click('Lock vault');
  expect(locks).toBe(2);
  expect(host.textContent).toContain('Authenticate to open a vault');
  expect(host.textContent).not.toContain('Authenticated vault workspace');
});

it('rejects an incorrect password without revealing the dashboard', async () => {
  vaults = [{ id, name: null, isLocked: true }];
  invalidPassword = true;
  await ready();
  await click('Decryption');
  await input('Vault password', 'incorrect password');
  await submit();
  expect(host.querySelector('[role="alert"]')?.textContent).toContain('Incorrect vault password');
  expect(host.querySelector<HTMLInputElement>('input[aria-label="Vault password"]')?.value).toBe('');
  expect(host.textContent).not.toContain('Encrypt and decrypt your files.');
});


it('requires exact confirmation before requesting native destructive deletion', async () => {
  vaults = [{ id, name: null, isLocked: true }];
  await ready();
  await click('Decryption');
  await click('Delete this vault');
  expect(deletedVaults).toBe(0);

  await input('Delete vault confirmation', `DELETE ${id}`);
  await click('Authenticate & permanently delete');

  expect(deletedVaults).toBe(1);
  expect(host.textContent).toContain('Vault permanently deleted from this device.');
});

it('shows and hides the creation password without changing its value', async () => {
  await ready();
  await click('Decryption');
  await input('Vault password', 'a unique disposable test password');
  const password = host.querySelector<HTMLInputElement>('input[aria-label="Vault password"]');
  expect(password?.type).toBe('password');
  await click('Show');
  expect(password?.type).toBe('text');
  expect(password?.value).toBe('a unique disposable test password');
  await click('Hide');
  expect(password?.type).toBe('password');
  expect(password?.value).toBe('a unique disposable test password');
});


it('starts both operation tabs with an empty password even with a development password configured', async () => {
  vi.stubEnv('DEV', true);
  vi.stubEnv('VITE_DEV_VAULT_PASSWORD', 'must-not-be-prefilled');
  await act(async () => { root.unmount(); });
  root = createRoot(host);
  await act(async () => { root.render(<App />); });
  await ready();

  for (const tab of ['Encryption', 'Decryption']) {
    await click(tab);
    const password = host.querySelector<HTMLInputElement>('input[aria-label="Vault password"]');
    expect(password).not.toBeNull();
    expect(password?.value).toBe('');
    expect(host.querySelector<HTMLButtonElement>('button[type="submit"]')?.disabled).toBe(true);
  }
});


it('does not offer biometric verification as a substitute for vault unlock', async () => {
  await ready();
  await click('Decryption');
  expect(host.textContent).not.toContain('Verify Touch ID');
  expect(host.textContent).toContain('Your password unlocks the vault.');
  expect(host.querySelector<HTMLInputElement>('input[aria-label="Vault password"]')).not.toBeNull();
});

it('keeps password access available if the biometric status read fails', async () => {
  await ready();
  vi.spyOn(native, 'getBiometricStatus').mockRejectedValue(new Error('status unavailable'));
  await click('Decryption');
  await input('Vault name', 'Personal'); await input('Vault password', 'correct horse battery staple'); await submit();
  expect(host.textContent).toContain('Authenticated vault workspace');
});
it('does not resurrect a nested plaintext editor after its details close', async () => {
  const nestedId = 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';
  const file = { id: 'protected-file', name: 'protected.txt', sizeBytes: 3 };
  vi.spyOn(native, 'listNestedVaults').mockResolvedValue([{ id: nestedId, fileCount: 1 }]);
  vi.spyOn(native, 'unlockNestedVault').mockResolvedValue([file]);
  let resolve!: (value: string) => void;
  vi.spyOn(native, 'readNestedTextFile').mockReturnValue(new Promise(done => { resolve = done; }));
  await ready(); await click('Decryption');
  await input('Vault name', 'Personal'); await input('Vault password', 'correct horse battery staple'); await submit();
  await click('Vault aaaaaaaa'); await input('Unlock nested vault PIN', '0427'); await submit();
  await click('protected.txt'); await input('Protected file operation PIN', '0427'); await click('Open text editor');
  await click('Close details');
  await act(async () => resolve('late protected plaintext'));
  expect(host.querySelector('[aria-label="Edit protected file contents"]')).toBeNull();
  expect(host.querySelector<HTMLTextAreaElement>('[aria-label="Nested file contents"]')?.value).toBe('');
  expect(host.textContent).not.toContain('Authenticated document opened');
});

it('does not republish plaintext after the root vault locks', async () => {
  let resolve!: (value: string) => void;
  vi.spyOn(native, 'readVaultTextFile').mockReturnValue(new Promise(done => { resolve = done; }));
  await ready(); await click('Decryption');
  await input('Vault name', 'Personal'); await input('Vault password', 'correct horse battery staple'); await submit();
  const edit = host.querySelector<HTMLButtonElement>('[aria-label="Edit private-plan.txt"]')!;
  await act(async () => edit.click());
  await click('Lock vault');
  await act(async () => resolve('late private plaintext'));
  expect(host.querySelector('[aria-label="File contents"]')).toBeNull();
  expect(host.textContent).not.toContain('private-plan.txt');
  expect(host.textContent).toContain('Authenticate to open a vault');
});

it('keeps late file metadata hidden while native locking is still pending', async () => {
  let resolveFiles!: (value: { id: string; name: string; sizeBytes: number }[]) => void;
  let resolveLock!: () => void;
  await ready(); await click('Decryption');
  vi.spyOn(native, 'listVaultFiles').mockReturnValue(new Promise(done => { resolveFiles = done; }));
  await input('Vault name', 'Personal'); await input('Vault password', 'correct horse battery staple'); await submit();
  vi.spyOn(native, 'lockVault').mockReturnValue(new Promise(done => { resolveLock = done; }));
  await click('Lock vault');
  await act(async () => resolveFiles([{ id: 'late-file', name: 'late-secret.txt', sizeBytes: 10 }]));
  expect(host.textContent).not.toContain('late-secret.txt');
  expect(host.textContent).toContain('Locking vault');
  await act(async () => resolveLock());
  expect(host.textContent).toContain('Authenticate to open a vault');
});
