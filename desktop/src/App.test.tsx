import { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { mockIPC } from '@tauri-apps/api/mocks';
import App from './App';
import type { SelectedFile } from './lib/native';

let root: Root;
let host: HTMLDivElement;
let selection: SelectedFile | null;
let fail = false;

beforeEach(async () => {
  vi.stubGlobal('isTauri', true);
  selection = { name: 'example.txt', sizeBytes: 104 };
  fail = false;
  mockIPC(command => {
    if (command === 'get_desktop_info') return { appName: 'Black Vault', version: '0.1.0', platform: 'macos', architecture: 'aarch64' };
    if (command === 'select_file') {
      if (fail) throw { code: 'file_unavailable', message: 'The selected file is unavailable.' };
      return selection;
    }
    throw new Error(`Unexpected command: ${command}`);
  });
  host = document.createElement('div'); document.body.appendChild(host);
  root = createRoot(host);
  await act(async () => { root.render(<App />); });
});
afterEach(async () => { await act(async () => root.unmount()); host.remove(); });

function button(label: string) {
  const result = Array.from(host.querySelectorAll('button')).find(element => element.textContent?.includes(label));
  if (!result) throw new Error(`Missing button: ${label}`);
  return result;
}
async function click(label: string) { await act(async () => { button(label).click(); }); }

it('loads native status, selects a file and clears the selection', async () => {
  expect(host.textContent).toContain('Native desktop');
  await click('Select a file');
  expect(host.textContent).toContain('example.txt');
  expect(host.textContent).toContain('104 bytes');
  await click('Clear selection');
  expect(host.textContent).not.toContain('example.txt');
  expect(host.querySelector('[role="status"]')?.textContent).toContain('Selection cleared');
});

it('preserves the current selection when the picker is cancelled', async () => {
  await click('Select a file');
  selection = null;
  await click('Select a file');
  expect(host.textContent).toContain('example.txt');
  expect(host.querySelector('[role="status"]')?.textContent).toContain('cancelled');
});

it('shows picker errors and allows another attempt', async () => {
  fail = true;
  await click('Select a file');
  expect(host.querySelector('[role="alert"]')?.textContent).toContain('unavailable');
  expect(button('Select a file').disabled).toBe(false);
  fail = false;
  await click('Select a file');
  expect(host.querySelector('[role="alert"]')).toBeNull();
  expect(host.textContent).toContain('example.txt');
});

it('renders filenames as text, never markup', async () => {
  selection = { name: '<img src=x onerror=alert(1)>.txt', sizeBytes: 0 };
  await click('Select a file');
  expect(host.textContent).toContain(selection.name);
  expect(host.querySelector('img')).toBeNull();
});
