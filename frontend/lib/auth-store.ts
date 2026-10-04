import type { User } from './types';
import { clearStoredAuth, readStoredAuth, writeStoredAuth } from './auth-storage';

type AuthState = { user: User | null; token: string | null; isLoading: boolean };
const initial: AuthState = { user: null, token: null, isLoading: true };
let snapshot = initial;
const listeners = new Set<() => void>();

function publish(next: AuthState) {
  if (next.token === snapshot.token && next.user?.id === snapshot.user?.id
    && next.user?.email === snapshot.user?.email && next.isLoading === snapshot.isLoading) return;
  snapshot = next;
  for (const listener of listeners) listener();
}
function reload() {
  publish({ ...readStoredAuth(() => window.localStorage), isLoading: false });
}
function storageChanged(event: StorageEvent) {
  if (event.key === null || event.key === 'auth_token' || event.key === 'auth_user') reload();
}

export const authStore = {
  getSnapshot: () => snapshot,
  // A stable empty snapshot keeps the server and first hydrated render consistent.
  getServerSnapshot: () => initial,
  subscribe(listener: () => void) {
    listeners.add(listener);
    if (listeners.size === 1) {
      window.addEventListener('storage', storageChanged);
      reload();
    }
    return () => {
      listeners.delete(listener);
      if (listeners.size === 0) window.removeEventListener('storage', storageChanged);
    };
  },
  login(token: string, user: User) {
    writeStoredAuth(() => window.localStorage, token, user);
    publish({ token, user, isLoading: false });
  },
  logout() {
    // Memory is cleared even if the browser denies access to persistent storage.
    clearStoredAuth(() => window.localStorage);
    publish({ token: null, user: null, isLoading: false });
  },
};
