import { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { mockIPC } from '@tauri-apps/api/mocks';
import TrustedContacts from './TrustedContacts';
import { native, type TrustedContact } from '@/lib/native';

let root: Root;
let host: HTMLDivElement;
let contacts: TrustedContact[];
const fingerprint = 'AAAAAAAA BBBBBBBB CCCCCCCC DDDDDDDD EEEEEEEE FFFFFFFF 11111111 22222222';
const changedFingerprint = '33333333 44444444 55555555 66666666 77777777 88888888 99999999 AAAAAAAA';

beforeEach(async () => {
  vi.stubGlobal('isTauri', true);
  contacts = [];
  mockIPC((command, args) => {
    if (command === 'get_public_identity') return { fingerprint, document: '{"magic":"BLACKVAULT-IDENTITY"}' };
    if (command === 'list_contacts') return contacts;
    if (command === 'import_contact_identity') {
      const name = String((args as { name: string }).name);
      const existing = contacts.find(contact => contact.name === name);
      const contact: TrustedContact = {
        name,
        fingerprint: changedFingerprint,
        trust: existing?.trust === 'verified' ? 'key_changed' : 'unverified',
        previousVerifiedFingerprint: existing?.trust === 'verified' ? existing.fingerprint : null,
      };
      contacts = contacts.filter(item => item.name !== name).concat(contact);
      return contact;
    }
    if (command === 'verify_contact_fingerprint') {
      const name = String((args as { name: string }).name);
      const contact = contacts.find(item => item.name === name);
      if (!contact) throw { code: 'contact_not_found', message: 'The contact could not be found.' };
      const updated = { ...contact, trust: 'verified' as const, previousVerifiedFingerprint: null };
      contacts = contacts.filter(item => item.name !== name).concat(updated);
      return updated;
    }
    if (command === 'revoke_contact') {
      const name = String((args as { name: string }).name);
      const contact = contacts.find(item => item.name === name);
      if (!contact) throw { code: 'contact_not_found', message: 'The contact could not be found.' };
      const updated = { ...contact, trust: 'revoked' as const, previousVerifiedFingerprint: null };
      contacts = contacts.filter(item => item.name !== name).concat(updated);
      return updated;
    }
    throw new Error('Unexpected command: ' + command);
  });
  host = document.createElement('div');
  document.body.appendChild(host);
  root = createRoot(host);
});

afterEach(async () => {
  await act(async () => root.unmount());
  host.remove();
  vi.unstubAllGlobals(); vi.restoreAllMocks();
});

async function render() {
  await act(async () => { root.render(<TrustedContacts />); });
  await act(async () => { await Promise.resolve(); });
}

async function input(label: string, value: string) {
  const element = host.querySelector<HTMLInputElement | HTMLTextAreaElement>(`[aria-label="${label}"]`);
  if (!element) throw new Error('Missing input: ' + label);
  const prototype = element instanceof HTMLTextAreaElement ? HTMLTextAreaElement.prototype : HTMLInputElement.prototype;
  const setter = Object.getOwnPropertyDescriptor(prototype, 'value')?.set;
  await act(async () => {
    setter?.call(element, value);
    element.dispatchEvent(new Event('input', { bubbles: true }));
  });
}

async function click(label: string) {
  const button = Array.from(host.querySelectorAll('button')).find(element => element.textContent?.includes(label));
  if (!button) throw new Error('Missing button: ' + label);
  await act(async () => { button.click(); });
}

it('shows only the public identity and starts with no trusted contacts', async () => {
  await render();
  expect(host.textContent).toContain(fingerprint);
  expect(host.textContent).toContain('Private encryption and signing keys remain in native secure storage');
  expect(host.textContent).toContain('No contacts imported yet.');
});

it('imports a contact as unverified and requires a fingerprint before verification', async () => {
  await render();
  await input('Contact name', 'Alice');
  await input('Public identity document', '{"magic":"BLACKVAULT-IDENTITY"}');
  await click('Import identity');

  expect(host.textContent).toContain('Alice');
  expect(host.textContent).toContain('Unverified');
  const verify = Array.from(host.querySelectorAll('button')).find(button => button.textContent?.includes('Verify fingerprint'));
  expect(verify?.disabled).toBe(true);

  await input('Verify fingerprint for Alice', changedFingerprint);
  await click('Verify fingerprint');
  expect(host.textContent).toContain('Verified');
});

it('makes a verified key replacement explicit and shows the previous fingerprint', async () => {
  contacts = [{
    name: 'Alice',
    fingerprint,
    trust: 'verified',
    previousVerifiedFingerprint: null,
  }];
  await render();

  await input('Contact name', 'Alice');
  await input('Public identity document', '{"magic":"BLACKVAULT-IDENTITY","changed":true}');
  await click('Import identity');

  expect(host.textContent).toContain('Key changed');
  expect(host.textContent).toContain('Previously verified identity changed');
  expect(host.textContent).toContain(fingerprint);
  expect(host.textContent).toContain(changedFingerprint);
});

it('keeps revoked contacts visible without a verification action', async () => {
  contacts = [{
    name: 'Alice',
    fingerprint,
    trust: 'verified',
    previousVerifiedFingerprint: null,
  }];
  await render();
  await click('Revoke trust');

  expect(host.textContent).toContain('Revoked');
  expect(host.textContent).toContain('This contact remains stored but is not trusted.');
  expect(host.querySelector('[aria-label="Verify fingerprint for Alice"]')).toBeNull();
});

it('keeps contacts available when the OS identity store fails', async () => {
  contacts = [{ name: 'Alice', fingerprint, trust: 'verified', previousVerifiedFingerprint: null }];
  vi.spyOn(native, 'getPublicIdentity').mockRejectedValue(new Error('Secure storage unavailable.'));
  await render();
  expect(host.textContent).toContain('Alice');
  expect(host.textContent).toContain('Identity unavailable.');
  expect(host.querySelector('[role="alert"]')?.textContent).toContain('Secure storage unavailable.');
});
it('distinguishes a failed contact read from an empty contact book', async () => {
  vi.spyOn(native, 'listContacts').mockRejectedValue(new Error('Contact storage unavailable.'));
  await render();
  expect(host.textContent).toContain(fingerprint);
  expect(host.textContent).toContain('Contact list unavailable.');
  expect(host.textContent).not.toContain('No contacts imported yet.');
});
