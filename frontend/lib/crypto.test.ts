import { describe, expect, it } from "vitest";
import { decryptVaultKey, encryptVaultKey, generateVaultKey } from "./crypto";

describe("Vault cryptography", () => {
  it("encrypts and decrypts a vault key with the correct password", () => {
    const vaultKey = generateVaultKey();
    const password = "correct-horse-battery-staple";

    const envelope = encryptVaultKey(vaultKey, password);
    const decryptedVaultKey = decryptVaultKey(envelope, password);

    expect(decryptedVaultKey).toEqual(vaultKey);
  });

  it("fails to decrypt with the wrong password", () => {
    const vaultKey = generateVaultKey();
    const password = "correct-password";
    const wrongPassword = "wrong-password";

    const envelope = encryptVaultKey(vaultKey, password);

    expect(() => {
      decryptVaultKey(envelope, wrongPassword);
    }).toThrow();
  });

  it("generates a unique salt and nonce for each encryption", () => {
    const vaultKey = generateVaultKey();
    const password = "test-password";

    const first = encryptVaultKey(vaultKey, password);
    const second = encryptVaultKey(vaultKey, password);

    expect(first.key_derivation_salt).not.toBe(second.key_derivation_salt);

    expect(first.encryption_nonce).not.toBe(second.encryption_nonce);
  });

  it("preserves the configured Argon2id parameters", () => {
    const vaultKey = generateVaultKey();
    const password = "test-password";

    const envelope = encryptVaultKey(vaultKey, password);

    expect(envelope.key_derivation_params.t_cost).toBe(2);
    expect(envelope.key_derivation_params.m_cost).toBe(19456);
    expect(envelope.key_derivation_params.p_cost).toBe(1);
    expect(envelope.key_derivation_params.output_len).toBe(32);
    expect(envelope.encryption_version).toBe("1");
  });

  it("generates 256-bit vault keys", () => {
    const vaultKey = generateVaultKey();

    expect(vaultKey).toHaveLength(32);
  });
});

it('rejects malformed vault metadata before attempting expensive derivation', () => {
  const envelope = encryptVaultKey(generateVaultKey(), 'password');
  for (const invalid of [
    { ...envelope, encryption_version: 'unknown' },
    { ...envelope, key_derivation_salt: 'mismatch' },
    { ...envelope, encryption_nonce: btoa('short') },
    { ...envelope, key_derivation_params: { ...envelope.key_derivation_params, m_cost: 2 ** 31 } },
  ]) expect(() => decryptVaultKey(invalid, 'password')).toThrow();
});
