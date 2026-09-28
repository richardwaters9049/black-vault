import { expect, it } from 'vitest';
import { generateVaultKey } from './crypto';
import { decryptFile, decryptFilename, encryptFile, MAX_FILE_SIZE } from './file-crypto';

it.each([new Uint8Array(), new Uint8Array([0, 255, 128, 42])])('encrypts contents, filename and individual file keys', data => {
  const vault = generateVaultKey();
  const first = encryptFile(data, '秘密.txt', vault);
  const second = encryptFile(data, '秘密.txt', vault);
  expect(first.encrypted_file_key).not.toBe(second.encrypted_file_key);
  expect(first.encryption_nonce).not.toBe(second.encryption_nonce);
  expect(decryptFilename(first, vault)).toBe('秘密.txt');
  const ciphertext = new Uint8Array(Buffer.from(first.encrypted_data, 'base64'));
  expect(decryptFile(first, ciphertext, vault)).toEqual(data);
  expect(() => decryptFile(first, ciphertext, generateVaultKey())).toThrow();
  ciphertext[ciphertext.length - 1] ^= 1;
  expect(() => decryptFile(first, ciphertext, vault)).toThrow();
});

it('rejects oversized files and unsupported legacy records', () => {
  const key = generateVaultKey();
  expect(() => encryptFile(new Uint8Array(MAX_FILE_SIZE + 1), 'large', key)).toThrow();
  const record = encryptFile(new Uint8Array(), 'name', key);
  expect(() => decryptFilename({ ...record, encryption_version: '1' }, key)).toThrow();
  expect(() => decryptFilename({ ...record, encrypted_filename: '!!!' }, key)).toThrow();
});
