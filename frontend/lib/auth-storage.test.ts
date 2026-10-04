import { expect, it, vi } from 'vitest';
import { clearStoredAuth, readStoredAuth, writeStoredAuth } from './auth-storage';

function storage() {
  const entries = new Map<string, string>();
  return {
    getItem: vi.fn((key: string) => entries.get(key) ?? null),
    setItem: vi.fn((key: string, value: string) => { entries.set(key, value); }),
    removeItem: vi.fn((key: string) => { entries.delete(key); }),
    clear: () => entries.clear(), key: () => null, get length() { return entries.size; },
  };
}
it('restores only a complete typed authentication record', () => {
  const value = storage(); const user = { id: 'test-id', email: 'user@example.test' };
  writeStoredAuth(() => value, 'test-token', user);
  expect(readStoredAuth(() => value)).toEqual({ token: 'test-token', user });
});
it('clears malformed, partial and wrongly shaped records', () => {
  for (const encoded of ['{broken', 'null', '{}', '{"id":42,"email":"a"}']) {
    const value = storage(); value.setItem('auth_token', 'test-token'); value.setItem('auth_user', encoded);
    expect(readStoredAuth(() => value)).toEqual({ token: null, user: null });
    expect(value.length).toBe(0);
  }
});
it('fails closed when the browser blocks storage access', () => {
  const blocked = () => { throw new Error('SecurityError'); };
  expect(readStoredAuth(blocked)).toEqual({ token: null, user: null });
  expect(clearStoredAuth(blocked)).toBe(false);
  expect(() => writeStoredAuth(blocked, 'test-token', { id: 'id', email: 'a@b.test' })).toThrow('Unable to save your sign-in');
});
it('cleans up a token if saving its user record fails', () => {
  const value = storage();
  value.setItem.mockImplementationOnce(value.setItem.getMockImplementation()!);
  value.setItem.mockImplementationOnce(() => { throw new Error('QuotaExceededError'); });
  expect(() => writeStoredAuth(() => value, 'token', { id: 'id', email: 'a@b.test' })).toThrow('Unable to save your sign-in');
  expect(value.removeItem).toHaveBeenCalledWith('auth_token');
  expect(value.removeItem).toHaveBeenCalledWith('auth_user');
  expect(value.length).toBe(0);
});
