import { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { mockIPC } from '@tauri-apps/api/mocks';
import VaultWorkspace from './VaultWorkspace';
import type { VaultSummary } from '@/lib/native';

let root: Root;
let host: HTMLDivElement;
let vaults: VaultSummary[];
let failUnlock = false;
const id = '1234567890abcdef1234567890abcdef';

beforeEach(async () => {
  vi.stubGlobal('isTauri', true);
  vaults = [];
  failUnlock = false;
  mockIPC((command, args) => {
    if (command === 'list_vaults') return vaults;
    if (command === 'create_vault') {
      vaults = [{ id, name: String((args as { name: string }).name), isLocked: false }];
      return vaults[0];
    }
    if (command === 'lock_vault') {
      vaults = vaults.map(vault => ({ ...vault, name: null, isLocked: true }));
      return null;
    }
    if (command === 'unlock_vault') {
      if (failUnlock) throw { code: 'invalid_credentials', message: 'Incorrect password or vault authentication failed.' };
      vaults = vaults.map(vault => ({ ...vault, name: 'Personal', isLocked: false }));
      return vaults[0];
    }
    throw new Error('Unexpected command');
  });
  host = document.createElement('div');
  document.body.appendChild(host);
  root = createRoot(host);
  await act(async () => { root.render(<VaultWorkspace ready />); });
});
afterEach(async () => { await act(async () => root.unmount()); host.remove(); vi.unstubAllGlobals(); });

function button(label: string): HTMLButtonElement {
  const result = Array.from(host.querySelectorAll('button')).find(element => element.textContent?.includes(label));
  if (!result) throw new Error('Missing button: ' + label);
  return result;
}
async function click(label: string) {
  await act(async () => { button(label).click(); });
}
async function input(id: string, value: string) {
  const element = host.querySelector<HTMLInputElement>('#' + id);
  if (!element) throw new Error('Missing input: ' + id);
  const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value')?.set;
  if (!setter) throw new Error('Missing value setter');
  await act(async () => { setter.call(element, value); element.dispatchEvent(new Event('input', { bubbles: true })); });
}
async function submit(index = 0) {
  const form = host.querySelectorAll<HTMLFormElement>('form')[index];
  if (!form) throw new Error('Missing form at index: ' + index);
  await act(async () => { form.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true })); });
}

it('creates a vault, hides its name on lock and reveals it after unlock', async () => {
  await input('vault-name', 'Personal');
  await input('vault-password', 'correct horse battery staple');
  await submit();
  expect(host.textContent).toContain('Personal');
  expect(host.textContent).toContain('Unlocked on this Mac');
  expect(host.querySelector<HTMLInputElement>('#vault-password')).toBeNull();

  await click('Lock vault');
  expect(host.textContent).toContain('Vault ' + id.slice(0, 8));
  expect(host.textContent).not.toContain('Unlocked on this Mac');
  await click('Unlock');
  await input('unlock-' + id, 'correct horse battery staple');
  await submit(1);
  expect(host.textContent).toContain('Unlocked on this Mac');
  expect(host.textContent).toContain('Personal');
});

it('shows authentication failures and clears the password input', async () => {
  vaults = [{ id, name: null, isLocked: true }];
  await act(async () => { root.render(<VaultWorkspace ready={false} />); });
  await act(async () => { root.render(<VaultWorkspace ready />); });
  failUnlock = true;
  await click('Unlock');
  await input('unlock-' + id, 'incorrect password');
  await submit(1);
  expect(host.querySelector('[role="alert"]')?.textContent).toContain('Incorrect password');
  expect(host.querySelector<HTMLInputElement>('#unlock-' + id)?.value).toBe('');
  expect(host.textContent).not.toContain('Unlocked on this Mac');
});
