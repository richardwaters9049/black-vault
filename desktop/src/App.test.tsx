import { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { mockIPC } from '@tauri-apps/api/mocks';
import App from './App';
import type { VaultSummary } from './lib/native';

const id = '1234567890abcdef1234567890abcdef';
let root: Root;
let host: HTMLDivElement;
let vaults: VaultSummary[];
let locks: number;
let invalidPassword: boolean;

beforeEach(async () => {
  vi.stubGlobal('isTauri', true);
  vaults = [];
  locks = 0;
  invalidPassword = false;
  mockIPC((command, args) => {
    if (command === 'get_desktop_info') return { appName: 'Black Vault', version: '0.1.0', platform: 'macos', architecture: 'aarch64' };
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
    if (command === 'list_vault_files') return [];
    throw new Error('Unexpected command: ' + command);
  });
  host = document.createElement('div');
  document.body.appendChild(host);
  root = createRoot(host);
  await act(async () => { root.render(<App />); });
});
afterEach(async () => { await act(async () => root.unmount()); host.remove(); vi.unstubAllGlobals(); });

async function ready() {
  await act(async () => { await new Promise(resolve => setTimeout(resolve, 1150)); });
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
  expect(host.textContent).toContain('Create your first vault');
  expect(host.textContent).not.toContain('Your vault, on your Mac.');
});

it('creates a vault, enters the dashboard and locks it on sign-out', async () => {
  await ready();
  await input('Vault name', 'Personal');
  await input('Vault password', 'correct horse battery staple');
  await submit();
  expect(host.textContent).toContain('Your vault, on your Mac.');
  await click('Sign out');
  expect(locks).toBe(2);
  expect(host.textContent).toContain('Unlock your workspace');
  expect(host.textContent).not.toContain('Your vault, on your Mac.');
});

it('rejects an incorrect password without revealing the dashboard', async () => {
  vaults = [{ id, name: null, isLocked: true }];
  invalidPassword = true;
  await ready();
  await input('Vault password', 'incorrect password');
  await submit();
  expect(host.querySelector('[role="alert"]')?.textContent).toContain('Incorrect vault password');
  expect(host.querySelector<HTMLInputElement>('input[aria-label="Vault password"]')?.value).toBe('');
  expect(host.textContent).not.toContain('Your vault, on your Mac.');
});
