import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { authStore } from './auth-store';

let value: Storage;
let windowEvents: EventTarget;
let unsubscribe: () => void;
beforeEach(() => {
  const entries = new Map<string, string>();
  value = {
    getItem: key => entries.get(key) ?? null,
    setItem: (key, content) => { entries.set(key, content); },
    removeItem: key => { entries.delete(key); },
    clear: () => entries.clear(), key: () => null, get length() { return entries.size; },
  };
  windowEvents = new EventTarget();
  vi.stubGlobal('window', {
    localStorage: value,
    addEventListener: windowEvents.addEventListener.bind(windowEvents),
    removeEventListener: windowEvents.removeEventListener.bind(windowEvents),
  });
  unsubscribe = authStore.subscribe(() => {});
});
afterEach(() => { authStore.logout(); unsubscribe(); vi.unstubAllGlobals(); vi.restoreAllMocks(); });
it('reflects sign-in and sign-out from another browser tab while keeping SSR empty', () => {
  const user = { id: 'id', email: 'user@example.test' };
  value.setItem('auth_token', 'token'); value.setItem('auth_user', JSON.stringify(user));
  const event = new Event('storage'); Object.defineProperty(event, 'key', { value: 'auth_user' });
  windowEvents.dispatchEvent(event);
  expect(authStore.getSnapshot()).toEqual({ token: 'token', user, isLoading: false });
  expect(authStore.getServerSnapshot()).toEqual({ token: null, user: null, isLoading: true });
  value.clear(); windowEvents.dispatchEvent(event);
  expect(authStore.getSnapshot().token).toBeNull();
});
it('does not publish sign-in when persistent storage rejects it', () => {
  vi.spyOn(value, 'setItem').mockImplementation(() => { throw new Error('blocked'); });
  expect(() => authStore.login('token', { id: 'id', email: 'user@example.test' })).toThrow('Unable to save');
  expect(authStore.getSnapshot().token).toBeNull();
});
