'use client';

import React, { createContext, useContext, useState, ReactNode } from 'react';
import { User } from './types';

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
  const getInitialAuthState = () => {
    if (typeof window === 'undefined') {
      return { user: null, token: null, isLoading: true };
    }

    const storedToken = localStorage.getItem('auth_token');
    const storedUser = localStorage.getItem('auth_user');

    if (storedToken && storedUser) {
      try {
        return {
          user: JSON.parse(storedUser),
          token: storedToken,
          isLoading: false,
        };
      } catch {
        console.warn('Stored authentication state was invalid and has been cleared');
        localStorage.removeItem('auth_token');
        localStorage.removeItem('auth_user');
        return { user: null, token: null, isLoading: false };
      }
    }

    return { user: null, token: null, isLoading: false };
  };

  const initialState = getInitialAuthState();
  const [user, setUser] = useState<User | null>(initialState.user);
  const [token, setToken] = useState<string | null>(initialState.token);
  const [isLoading] = useState(initialState.isLoading);

  const login = (newToken: string, newUser: User) => {
    setToken(newToken);
    setUser(newUser);
    localStorage.setItem('auth_token', newToken);
    localStorage.setItem('auth_user', JSON.stringify(newUser));
  };

  const logout = () => {
    setToken(null);
    setUser(null);
    localStorage.removeItem('auth_token');
    localStorage.removeItem('auth_user');
  };

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
