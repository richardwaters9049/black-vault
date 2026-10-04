import { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { native } from '@/lib/native';
import DashboardOverview from './DashboardOverview';

let host: HTMLDivElement;
let root: Root;
beforeEach(() => { host = document.createElement('div'); document.body.appendChild(host); root = createRoot(host); });
afterEach(async () => { await act(async () => root.unmount()); host.remove(); vi.restoreAllMocks(); });
it('retains available vaults when contacts fail and does not invent zero trust counts', async () => {
  vi.spyOn(native, 'listVaults').mockResolvedValue([{ id: 'vault-id', name: null, isLocked: true }]);
  vi.spyOn(native, 'listContacts').mockRejectedValue(new Error('Contact storage unavailable.'));
  await act(async () => root.render(<DashboardOverview currentVault={null} files={[]} onCreateVault={() => {}}/>));
  expect(host.textContent).toContain('vault-id');
  expect(host.textContent).toContain('Contact list unavailable.');
  expect(host.textContent).not.toContain('Create your first vault');
  expect(host.textContent).not.toContain('No contacts imported yet.');
});
it('does not offer first-vault creation when the vault list is unavailable', async () => {
  vi.spyOn(native, 'listVaults').mockRejectedValue(new Error('Vault storage unavailable.'));
  vi.spyOn(native, 'listContacts').mockResolvedValue([]);
  await act(async () => root.render(<DashboardOverview currentVault={null} files={[]} onCreateVault={() => {}}/>));
  expect(host.textContent).toContain('Vault list unavailable.');
  expect(host.textContent).not.toContain('Create your first vault');
});
