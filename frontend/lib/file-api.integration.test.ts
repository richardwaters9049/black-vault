import { randomUUID } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { expect, it } from 'vitest';
import { decryptVaultKey, encryptVaultKey, generateVaultKey } from './crypto';
import { decryptFile, decryptFilename, encryptFile, type FileRecord } from './file-crypto';

// Opt in against a disposable local database. Account IDs are reported for cleanup.
const base = process.env.BLACK_VAULT_TEST_API;
it.skipIf(!base)('persists a browser envelope and downloads/decrypts the exact file through the Rust API', async () => {
  const credentials = { email: `file-e2e-${randomUUID()}@blackvault.local`, password: 'DisposableTest28Sept!' };
  let response = await fetch(`${base}/auth/register`, {
    method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(credentials),
  });
  expect(response.status).toBe(201);
  const user = await response.json();
  console.info(`Disposable integration account: ${user.id}`);
  response = await fetch(`${base}/auth/login`, {
    method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(credentials),
  });
  expect(response.status).toBe(200);
  const { token } = await response.json();
  const headers = { Authorization: `Bearer ${token}`, 'Content-Type': 'application/json' };
  const key = generateVaultKey();
  let fileId: string | undefined;
  try {
    const envelope = encryptVaultKey(key, 'separate-vault-password');
    response = await fetch(`${base}/vault/setup`, { method: 'POST', headers, body: JSON.stringify(envelope) });
    expect(response.status).toBe(201);
    response = await fetch(`${base}/vault/key`, { headers });
    const restored = decryptVaultKey(await response.json(), 'separate-vault-password');
    expect(restored).toEqual(key);
    restored.fill(0);
    const original = new Uint8Array(readFileSync(new URL('../../tests/fixtures/round-trip.txt', import.meta.url)));
    const encrypted = encryptFile(original, 'round-trip.txt', key);
    expect(JSON.stringify(encrypted)).not.toContain('Disposable encrypted');
    response = await fetch(`${base}/files/encrypted`, { method: 'POST', headers, body: JSON.stringify(encrypted) });
    expect(response.status).toBe(201);
    const record: FileRecord = await response.json();
    fileId = record.id;
    response = await fetch(`${base}/files/${record.id}/data`, { headers });
    expect(response.status).toBe(200);
    const ciphertext = new Uint8Array(await response.arrayBuffer());
    expect(ciphertext).not.toEqual(original);
    expect(decryptFilename(record, key)).toBe('round-trip.txt');
    const plaintext = decryptFile(record, ciphertext, key);
    expect(plaintext).toEqual(original);
    plaintext.fill(0);
    response = await fetch(`${base}/files/${record.id}/data`);
    expect(response.status).toBe(401);
  } finally {
    key.fill(0);
    if (fileId) {
      response = await fetch(`${base}/files/${fileId}`, { method: 'DELETE', headers });
      expect(response.status).toBe(204);
    }
  }
}, 20_000);
