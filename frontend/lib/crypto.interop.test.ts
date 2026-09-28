import { execFileSync } from 'node:child_process';
import { resolve } from 'node:path';
import { beforeAll, describe, expect, it } from 'vitest';
import { gcm } from '@noble/ciphers/aes.js';
import vectors from '../../tests/fixtures/crypto-v1.json';
import { decrypt, deriveKey, encrypt } from './crypto';

const backend = resolve(import.meta.dirname, '../../backend');
const bridge = resolve(backend, 'target/debug/examples/crypto_interop');
const bytes = (value: string) => new Uint8Array(Buffer.from(value, 'base64'));
const base64 = (value: Uint8Array) => Buffer.from(value).toString('base64');
const rust = (request: object) => JSON.parse(execFileSync(bridge, {
  input: JSON.stringify(request), encoding: 'utf8',
}));

beforeAll(() => {
  execFileSync('cargo', ['build', '--locked', '--example', 'crypto_interop'], {
    cwd: backend, stdio: 'pipe',
  });
}, 120_000);

describe('Rust ↔ TypeScript compatibility', () => {
  for (const vector of vectors) {
    it(`matches the shared vector and exchanges ciphertext: ${vector.password}`, () => {
      const key = deriveKey(vector.password, vector.params);
      expect(base64(key)).toBe(vector.key);
      expect(base64(gcm(key, bytes(vector.encrypted.nonce)).encrypt(bytes(vector.plaintext))))
        .toBe(vector.encrypted.ciphertext);
      expect(base64(decrypt(vector.encrypted, key))).toBe(vector.plaintext);
      const fromRust = rust(vector);
      expect(fromRust.key).toBe(vector.key);
      expect(fromRust.plaintext).toBe(vector.plaintext);
      const rustEncrypted = rust({ ...vector, encrypted: undefined });
      expect(base64(decrypt(rustEncrypted.encrypted, key))).toBe(vector.plaintext);
      const tsEncrypted = encrypt(bytes(vector.plaintext), key);
      expect(rust({ ...vector, encrypted: tsEncrypted }).plaintext).toBe(vector.plaintext);
    });
  }
  it('rejects wrong passwords, damaged ciphertext, tags and malformed nonces in both languages', () => {
    const vector = vectors[0];
    const wrongKey = deriveKey('incorrect', vector.params);
    expect(() => decrypt(vector.encrypted, wrongKey)).toThrow();
    expect(rust({ ...vector, password: 'incorrect' }).rejected).toBe(true);
    const corrupted = (offset: number) => {
      const data = bytes(vector.encrypted.ciphertext);
      data[offset] ^= 1;
      return { ...vector.encrypted, ciphertext: base64(data) };
    };
    for (const encrypted of [corrupted(0), corrupted(47),
      { ...vector.encrypted, nonce: base64(new Uint8Array(11)) },
      { ...vector.encrypted, nonce: '!!!' }]) {
      expect(() => decrypt(encrypted, bytes(vector.key))).toThrow();
      expect(rust({ ...vector, encrypted }).rejected).toBe(true);
    }
  });
});
