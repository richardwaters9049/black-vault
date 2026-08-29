import { argon2id } from "@noble/hashes/argon2.js";
import { utf8ToBytes } from "@noble/hashes/utils.js";
import { gcm } from "@noble/ciphers/aes.js";
import { randomBytes } from "@noble/ciphers/utils.js";

export interface DerivationParams {
  salt: string;
  t_cost: number;
  m_cost: number;
  p_cost: number;
  output_len: number;
}

export interface EncryptedData {
  ciphertext: string;
  nonce: string;
}

export interface VaultKeyEnvelope {
  encrypted_vault_key: string;
  key_derivation_salt: string;
  key_derivation_params: DerivationParams;
  encryption_nonce: string;
  encryption_version: string;
}

const VAULT_KEY_SIZE = 32;
const FILE_KEY_SIZE = 32;
const NONCE_SIZE = 12;

export function generateVaultKey(): Uint8Array {
  return randomBytes(VAULT_KEY_SIZE);
}

export function generateFileKey(): Uint8Array {
  return randomBytes(FILE_KEY_SIZE);
}

export function generateDerivationParams(): DerivationParams {
  const salt = randomBytes(16);
  const saltBase64 = bufferToBase64(salt);

  return {
    salt: saltBase64,
    t_cost: 2,
    m_cost: 19456,
    p_cost: 1,
    output_len: VAULT_KEY_SIZE,
  };
}

export function deriveKey(
  password: string,
  params: DerivationParams,
): Uint8Array {
  const salt = base64ToBuffer(params.salt);
  const passwordBytes = utf8ToBytes(password);

  return argon2id(passwordBytes, salt, {
    t: params.t_cost,
    m: params.m_cost,
    p: params.p_cost,
    dkLen: params.output_len,
  });
}

export function encrypt(plaintext: Uint8Array, key: Uint8Array): EncryptedData {
  const nonce = randomBytes(NONCE_SIZE);
  const aes = gcm(key, nonce);

  const ciphertext = aes.encrypt(plaintext);

  return {
    ciphertext: bufferToBase64(ciphertext),
    nonce: bufferToBase64(nonce),
  };
}

export function decrypt(encrypted: EncryptedData, key: Uint8Array): Uint8Array {
  const nonce = base64ToBuffer(encrypted.nonce);
  const ciphertext = base64ToBuffer(encrypted.ciphertext);

  const aes = gcm(key, nonce);

  return aes.decrypt(ciphertext);
}

export function encryptVaultKey(
  vaultKey: Uint8Array,
  vaultPassword: string,
): VaultKeyEnvelope {
  const params = generateDerivationParams();
  const derivedKey = deriveKey(vaultPassword, params);
  const encrypted = encrypt(vaultKey, derivedKey);

  return {
    encrypted_vault_key: encrypted.ciphertext,
    key_derivation_salt: params.salt,
    key_derivation_params: params,
    encryption_nonce: encrypted.nonce,
    encryption_version: "1",
  };
}

export function decryptVaultKey(
  envelope: VaultKeyEnvelope,
  vaultPassword: string,
): Uint8Array {
  const derivedKey = deriveKey(vaultPassword, envelope.key_derivation_params);

  const encrypted: EncryptedData = {
    ciphertext: envelope.encrypted_vault_key,
    nonce: envelope.encryption_nonce,
  };

  return decrypt(encrypted, derivedKey);
}

function bufferToBase64(buffer: Uint8Array): string {
  const bytes = new Uint8Array(buffer);
  let binary = "";

  for (let i = 0; i < bytes.byteLength; i++) {
    binary += String.fromCharCode(bytes[i]);
  }

  return btoa(binary);
}

function base64ToBuffer(base64: string): Uint8Array {
  const binary = atob(base64);
  const bytes = new Uint8Array(binary.length);

  for (let i = 0; i < binary.length; i++) {
    bytes[i] = binary.charCodeAt(i);
  }

  return bytes;
}
