import type { User } from './types';

type StorageProvider = () => Storage;
const TOKEN = 'auth_token';
const USER = 'auth_user';

export function clearStoredAuth(storage: StorageProvider): boolean {
  try {
    const value = storage();
    value.removeItem(TOKEN); value.removeItem(USER);
    return true;
  } catch { return false; }
}

export function readStoredAuth(storage: StorageProvider): { token: string | null; user: User | null } {
  try {
    const value = storage();
    const token = value.getItem(TOKEN);
    const encoded = value.getItem(USER);
    const user: unknown = encoded ? JSON.parse(encoded) : null;
    if (token?.trim() && typeof user === 'object' && user !== null
      && 'id' in user && typeof user.id === 'string' && user.id.trim()
      && 'email' in user && typeof user.email === 'string' && user.email.trim()) {
      return { token, user: { id: user.id, email: user.email } };
    }
  } catch { /* Malformed or blocked storage must never create an authenticated state. */ }
  clearStoredAuth(storage);
  return { token: null, user: null };
}

export function writeStoredAuth(storage: StorageProvider, token: string, user: User): void {
  try {
    const value = storage();
    value.setItem(TOKEN, token); value.setItem(USER, JSON.stringify(user));
  } catch {
    // Clean up a partial write before reporting a sign-in failure.
    clearStoredAuth(storage);
    throw new Error('Unable to save your sign-in. Allow browser storage and try again.');
  }
}
