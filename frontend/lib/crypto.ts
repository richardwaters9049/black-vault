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
  if (salt.length !== 16 || params.t_cost !== 2 || params.m_cost !== 19456 ||
      params.p_cost !== 1 || params.output_len !== VAULT_KEY_SIZE) {
    throw new Error("Unsupported derivation parameters");
  }
  const passwordBytes = utf8ToBytes(password);

  try {
    return argon2id(passwordBytes, salt, {
      t: params.t_cost,
      m: params.m_cost,
      p: params.p_cost,
      dkLen: params.output_len,
    });
  } finally {
    passwordBytes.fill(0);
  }
}

export function encrypt(plaintext: Uint8Array, key: Uint8Array, additionalData?: Uint8Array): EncryptedData {
  if (key.length !== VAULT_KEY_SIZE) throw new Error("Invalid key length");
  const nonce = randomBytes(NONCE_SIZE);
  const aes = gcm(key, nonce, additionalData);

  const ciphertext = aes.encrypt(plaintext);

  return {
    ciphertext: bufferToBase64(ciphertext),
    nonce: bufferToBase64(nonce),
  };
}

export function decrypt(encrypted: EncryptedData, key: Uint8Array, additionalData?: Uint8Array): Uint8Array {
  if (key.length !== VAULT_KEY_SIZE) throw new Error("Invalid key length");
  const nonce = base64ToBuffer(encrypted.nonce);
  if (nonce.length !== NONCE_SIZE) throw new Error("Invalid nonce length");
  const ciphertext = base64ToBuffer(encrypted.ciphertext);

  if (ciphertext.length < 16) throw new Error("Invalid ciphertext length");
  const aes = gcm(key, nonce, additionalData);

  return aes.decrypt(ciphertext);
}

export function encryptVaultKey(
  vaultKey: Uint8Array,
  vaultPassword: string,
): VaultKeyEnvelope {
  if (vaultKey.length !== VAULT_KEY_SIZE) throw new Error("Invalid vault key length");
  const params = generateDerivationParams();
  const derivedKey = deriveKey(vaultPassword, params);
  try {
    const encrypted = encrypt(vaultKey, derivedKey);

    return {
      encrypted_vault_key: encrypted.ciphertext,
      key_derivation_salt: params.salt,
      key_derivation_params: params,
      encryption_nonce: encrypted.nonce,
      encryption_version: "1",
    };
  } finally {
    derivedKey.fill(0);
  }
}

export function decryptVaultKey(
  envelope: VaultKeyEnvelope,
  vaultPassword: string,
): Uint8Array {
  if (envelope.encryption_version !== "1" ||
      envelope.key_derivation_salt !== envelope.key_derivation_params.salt ||
      base64ToBuffer(envelope.encrypted_vault_key).length !== 48) {
    throw new Error("Invalid vault envelope metadata");
  }
  const derivedKey = deriveKey(vaultPassword, envelope.key_derivation_params);

  const encrypted: EncryptedData = {
    ciphertext: envelope.encrypted_vault_key,
    nonce: envelope.encryption_nonce,
  };

  try {
    return decrypt(encrypted, derivedKey);
  } finally {
    derivedKey.fill(0);
  }
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
  if (btoa(binary) !== base64) throw new Error("Invalid standard Base64 encoding");
  const bytes = new Uint8Array(binary.length);

  for (let i = 0; i < binary.length; i++) {
    bytes[i] = binary.charCodeAt(i);
  }

  return bytes;
}
