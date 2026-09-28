import { decryptVaultKey, encryptVaultKey, generateVaultKey, type VaultKeyEnvelope } from './crypto';

/** Owns the accessible key buffer. Locking also invalidates pending operations. */
export class VaultSession {
  private key: Uint8Array | null = null;
  private generation = 0;

  get unlocked(): boolean { return this.key !== null; }

  lock(): void {
    this.generation += 1;
    this.key?.fill(0);
    this.key = null;
  }

  async unlock(password: string, retrieve: () => Promise<VaultKeyEnvelope>): Promise<boolean> {
    this.lock();
    const generation = this.generation;
    const envelope = await retrieve();
    if (generation !== this.generation) return false;
    const key = decryptVaultKey(envelope, password);
    if (generation !== this.generation) { key.fill(0); return false; }
    this.key = key;
    return true;
  }

  async create(password: string, persist: (envelope: VaultKeyEnvelope) => Promise<unknown>): Promise<boolean> {
    this.lock();
    const generation = this.generation;
    const key = generateVaultKey();
    try {
      const envelope = encryptVaultKey(key, password);
      // Persist only the envelope; unlock from a fresh retrieval afterwards.
      key.fill(0);
      await persist(envelope);
      return generation === this.generation;
    } finally {
      key.fill(0);
    }
  }

  withKey<T>(operation: (key: Uint8Array) => T): T {
    if (!this.key) throw new Error('Vault is locked');
    return operation(this.key);
  }
}
