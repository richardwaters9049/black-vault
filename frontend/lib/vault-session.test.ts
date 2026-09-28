import { describe, expect, it } from 'vitest';
import { VaultSession } from './vault-session';
import { encryptVaultKey, generateVaultKey, type VaultKeyEnvelope } from './crypto';

describe('Vault lifecycle', () => {
  it('creates, retrieves and repeatedly unlocks/locks, wiping accessible buffers', async () => {
    const session = new VaultSession();
    let stored: VaultKeyEnvelope;
    expect(await session.create('vault-password', async envelope => { stored = envelope; })).toBe(true);
    expect(session.unlocked).toBe(false);
    for (let i = 0; i < 2; i++) {
      expect(await session.unlock('vault-password', async () => stored!)).toBe(true);
      const buffer = session.withKey(key => key);
      expect(buffer.some(byte => byte !== 0)).toBe(true);
      session.lock();
      expect(buffer.every(byte => byte === 0)).toBe(true);
      expect(() => session.withKey(key => key)).toThrow('locked');
    }
    await expect(session.unlock('incorrect', async () => stored!)).rejects.toThrow();
    expect(session.unlocked).toBe(false);
  });

  it('does not restore a key when logout/unmount invalidates a pending unlock', async () => {
    const session = new VaultSession();
    const envelope = encryptVaultKey(generateVaultKey(), 'password');
    let resolve!: (value: VaultKeyEnvelope) => void;
    const pending = session.unlock('password', () => new Promise(done => { resolve = done; }));
    session.lock();
    resolve(envelope);
    expect(await pending).toBe(false);
    expect(session.unlocked).toBe(false);
  });

  it('stays locked after a failed or duplicate setup', async () => {
    const session = new VaultSession();
    await expect(session.create('password', async () => { throw new Error('409'); })).rejects.toThrow('409');
    expect(session.unlocked).toBe(false);
  });
});
