import { decrypt, encrypt, generateFileKey, type EncryptedData } from './crypto';

export const MAX_FILE_SIZE = 5 * 1024 * 1024;
const encoder = new TextEncoder();
const context = (purpose: string) => encoder.encode(`black-vault:file:v2:${purpose}`);
const decoder = new TextDecoder('utf-8', { fatal: true });
const toBase64 = (bytes: Uint8Array): string => {
  let text = '';
  for (let offset = 0; offset < bytes.length; offset += 8192) {
    text += String.fromCharCode(...bytes.subarray(offset, offset + 8192));
  }
  return btoa(text);
};
const fromBase64 = (text: string): Uint8Array => {
  const binary = atob(text);
  if (btoa(binary) !== text) throw new Error('Invalid Base64');
  return Uint8Array.from(binary, char => char.charCodeAt(0));
};
// Version 2 packs each independent nonce before its ciphertext and GCM tag.
const pack = (encrypted: EncryptedData): string => {
  const nonce = fromBase64(encrypted.nonce);
  const ciphertext = fromBase64(encrypted.ciphertext);
  const packed = new Uint8Array(nonce.length + ciphertext.length);
  packed.set(nonce);
  packed.set(ciphertext, nonce.length);
  return toBase64(packed);
};
const unpack = (packed: string): EncryptedData => {
  const bytes = fromBase64(packed);
  if (bytes.length < 28) throw new Error('Invalid encrypted envelope');
  return { nonce: toBase64(bytes.subarray(0, 12)), ciphertext: toBase64(bytes.subarray(12)) };
};

export interface EncryptedFileMetadata {
  encrypted_filename: string;
  encrypted_file_key: string;
  encryption_nonce: string;
  encryption_version: string;
  file_size: number;
}
export interface FileRecord extends EncryptedFileMetadata { id: string }
export interface FileUpload extends EncryptedFileMetadata { encrypted_data: string }

export function encryptFile(data: Uint8Array, filename: string, vaultKey: Uint8Array): FileUpload {
  if (data.length > MAX_FILE_SIZE) throw new Error('Files must be 5 MiB or smaller');
  const name = encoder.encode(filename);
  if (name.length === 0 || name.length > 1024) throw new Error('Invalid filename length');
  const fileKey = generateFileKey();
  try {
    const contents = encrypt(data, fileKey, context('contents'));
    let filenameEnvelope = encrypt(name, fileKey, context('filename'));
    while (filenameEnvelope.nonce === contents.nonce) filenameEnvelope = encrypt(name, fileKey, context('filename'));
    return {
      encrypted_filename: pack(filenameEnvelope),
      encrypted_file_key: pack(encrypt(fileKey, vaultKey, context('wrapped-key'))),
      encryption_nonce: contents.nonce,
      encryption_version: '2',
      file_size: data.length,
      encrypted_data: contents.ciphertext,
    };
  } finally { fileKey.fill(0); name.fill(0); }
}

function withFileKey<T>(record: EncryptedFileMetadata, vaultKey: Uint8Array, action: (key: Uint8Array) => T): T {
  if (record.encryption_version !== '2') throw new Error('Unsupported file encryption version');
  const key = decrypt(unpack(record.encrypted_file_key), vaultKey, context('wrapped-key'));
  try {
    if (key.length !== 32) throw new Error('Invalid file key');
    return action(key);
  } finally { key.fill(0); }
}

export function decryptFilename(record: EncryptedFileMetadata, vaultKey: Uint8Array): string {
  return withFileKey(record, vaultKey, key => {
    const name = decrypt(unpack(record.encrypted_filename), key, context('filename'));
    try { return decoder.decode(name); } finally { name.fill(0); }
  });
}

export function decryptFile(record: EncryptedFileMetadata, ciphertext: Uint8Array, vaultKey: Uint8Array): Uint8Array {
  if (!Number.isSafeInteger(record.file_size) || record.file_size < 0 || record.file_size > MAX_FILE_SIZE ||
      ciphertext.length !== record.file_size + 16) throw new Error('Invalid encrypted file size');
  return withFileKey(record, vaultKey, key => decrypt({
    nonce: record.encryption_nonce, ciphertext: toBase64(ciphertext),
  }, key, context('contents')));
}
