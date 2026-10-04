'use client';

import React, { createContext, useContext, useSyncExternalStore, ReactNode } from 'react';
import { User } from './types';
import { authStore } from './auth-store';

interface AuthContextType {
  user: User | null;
  token: string | null;
  login: (token: string, user: User) => void;
  logout: () => void;
  isAuthenticated: boolean;
  isLoading: boolean;
}

const AuthContext = createContext<AuthContextType | undefined>(undefined);

export function AuthProvider({ children }: { children: ReactNode }) {
  const { user, token, isLoading } = useSyncExternalStore(
    authStore.subscribe, authStore.getSnapshot, authStore.getServerSnapshot,
  );
  const login = authStore.login;
  const logout = authStore.logout;

  const value = {
    user,
    token,
    login,
    logout,
    isAuthenticated: !!token,
    isLoading,
  };

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
}

export function useAuth() {
  const context = useContext(AuthContext);
  if (context === undefined) {
    throw new Error('useAuth must be used within an AuthProvider');
  }
  return context;
}
