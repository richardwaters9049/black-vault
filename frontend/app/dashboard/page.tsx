'use client';

import { useEffect, useState, useRef } from 'react';
import { useRouter } from 'next/navigation';
import { useAuth } from '@/lib/auth-context';
import { api, ApiError } from '@/lib/api';
import { VaultFiles } from '@/components/vault-files';
import { VaultSession } from '@/lib/vault-session';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';

export default function DashboardPage() {
  const router = useRouter();
  const { user, token, logout, isAuthenticated, isLoading } = useAuth();
  const [session] = useState(() => new VaultSession());
  const operation = useRef(0);
  const [hasVault, setHasVault] = useState<boolean | null>(null);
  const [confirmPassword, setConfirmPassword] = useState('');
  const [apiError, setApiError] = useState('');
  const [isVerifying, setIsVerifying] = useState(true);

  const [isVaultUnlocked, setIsVaultUnlocked] = useState(false);
  const [vaultPassword, setVaultPassword] = useState('');
  const [vaultError, setVaultError] = useState('');
  const [isUnlocking, setIsUnlocking] = useState(false);

  useEffect(() => {
    let isMounted = true;
    const loadVault = async () => {
      try {
        await api.getVaultKey(token!);
        if (isMounted) setHasVault(true);
      } catch (error) {
        if (!isMounted) return;
        if (error instanceof ApiError && error.status === 404) setHasVault(false);
        else throw error;
      }
    };

    const verifyToken = async () => {
      if (!token) {
        return;
      }

      try {
        await api.getCurrentUser(token);
        await loadVault();
        if (isMounted) {
          setApiError('');
        }
      } catch (error) {
        if (isMounted) {
          session.lock();
          if (error instanceof ApiError && error.status === 401) {
            logout();
            router.push('/login');
          } else {
            setApiError('Unable to load your vault. Reload the page to retry.');
          }
        }
      } finally {
        if (isMounted) {
          setIsVerifying(false);
        }
      }
    };

    if (!isLoading && !isAuthenticated) {
      router.push('/login');
      return;
    }

    if (isAuthenticated && token) {
      verifyToken();
    } else if (isAuthenticated && !token) {
      // Invalid state - authenticated but no token
      logout();
      router.push('/login');
    }

    return () => {
      isMounted = false;
      operation.current += 1;
      session.lock();
    };
  }, [isAuthenticated, isLoading, token, router, logout, session]);

  const handleLogout = () => {
    handleLockVault();
    logout();
    router.push('/login');
  };

  const handleUnlockVault = async (e: React.FormEvent) => {
    e.preventDefault();
    setVaultError('');

    if (!vaultPassword) {
      setVaultError('Vault password is required');
      return;
    }

    if (hasVault === null || !token) return;
    if (!hasVault && (vaultPassword.length < 12 || vaultPassword !== confirmPassword)) {
      setVaultError('Use at least 12 characters and matching passwords.');
      return;
    }
    const currentOperation = ++operation.current;
    setIsUnlocking(true);
    const password = vaultPassword;
    setVaultPassword('');
    setConfirmPassword('');
    try {
      if (!hasVault) {
        const created = await session.create(password, envelope => api.setupVault(token, envelope));
        if (!created || currentOperation !== operation.current) return;
        setHasVault(true);
      }
      const unlocked = await session.unlock(password, () => api.getVaultKey(token));
      if (currentOperation === operation.current) setIsVaultUnlocked(unlocked);
    } catch (error) {
      if (currentOperation !== operation.current) return;
      if (error instanceof ApiError && error.status === 409) {
        setHasVault(true);
        setVaultError('A vault already exists. Unlock it with its original password.');
      } else {
        setVaultError('Unable to open the vault. Check your password and connection.');
      }
    } finally {
      if (currentOperation === operation.current) setIsUnlocking(false);
    }
  };

  const handleLockVault = () => {
    operation.current += 1;
    session.lock();
    setIsVaultUnlocked(false);
    setIsUnlocking(false);
    setVaultPassword('');
    setConfirmPassword('');
  };

  if (isLoading || isVerifying) {
    return (
      <div className="py-24 flex items-center justify-center bg-gradient-to-br from-slate-950 via-slate-900 to-slate-950">
        <div className="text-slate-400">Loading...</div>
      </div>
    );
  }

  if (!isAuthenticated) {
    return null;
  }

  return (
    <div className="pb-12 bg-gradient-to-br from-slate-950 via-slate-900 to-slate-950">
      <nav className="border-b border-slate-800 bg-slate-900/50 backdrop-blur">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
          <div className="flex justify-between items-center h-16">
            <div className="flex items-center">
              <h1 className="text-xl font-bold text-white">Black Vault</h1>
            </div>
            <div className="flex items-center space-x-4">
              <span className="text-slate-300 text-sm">{user?.email}</span>
              <Button
                onClick={handleLogout}
                variant="outline"
                className="border-slate-700 text-white hover:bg-white hover:text-blue-600 bg-blue-600 hover:cursor-pointer py-5 px-3"
              >
                Sign out
              </Button>
            </div>
          </div>
        </div>
      </nav>

      <main className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        {apiError && (
          <div className="mb-6 text-sm text-red-400 bg-red-950/30 border border-red-900/50 p-4 rounded">
            {apiError}
          </div>
        )}

        <div className="mb-8">
          <h2 className="text-2xl font-bold text-white mb-2">Dashboard</h2>
          <p className="text-slate-400">Welcome to your secure encryption workspace</p>
        </div>

        {!isVaultUnlocked ? (
          <Card className="border-slate-800 bg-slate-900/50 backdrop-blur mb-8">
            <CardHeader>
              <CardTitle className="text-white">{hasVault === null ? 'Vault unavailable' : hasVault ? 'Unlock Vault' : 'Create Vault'}</CardTitle>
              <CardDescription className="text-slate-400">
                {hasVault ? 'Enter your vault password to access encrypted features' : 'Choose a separate vault password. It cannot be recovered if forgotten.'}
              </CardDescription>
            </CardHeader>
            <CardContent>
              <form onSubmit={handleUnlockVault} className="space-y-4">
                <div className="space-y-2">
                  <Label htmlFor="vaultPassword" className="text-sm font-medium text-slate-200">
                    Vault password
                  </Label>
                  <Input
                    id="vaultPassword"
                    type="password"
                    placeholder="Enter your vault password"
                    value={vaultPassword}
                    onChange={(e) => setVaultPassword(e.target.value)}
                    disabled={isUnlocking || hasVault === null}
                    className="border-slate-700 bg-slate-800 text-white placeholder:text-slate-500"
                  />
                </div>
                {!hasVault && (
                  <div className="space-y-2">
                    <Label htmlFor="confirmPassword">Confirm vault password</Label>
                    <Input id="confirmPassword" type="password" autoComplete="new-password"
                      value={confirmPassword} onChange={e => setConfirmPassword(e.target.value)}
                      disabled={isUnlocking || hasVault === null} className="border-slate-700 bg-slate-800 text-white" />
                  </div>
                )}
                {vaultError && (
                  <div className="text-sm text-red-400 bg-red-950/30 border border-red-900/50 p-3 rounded">
                    {vaultError}
                  </div>
                )}
                <Button
                  type="submit"
                  disabled={isUnlocking || hasVault === null}
                  className="bg-blue-600 hover:bg-blue-700 text-white"
                >
                  {isUnlocking ? 'Opening vault...' : hasVault ? 'Unlock Vault' : 'Create Vault'}
                </Button>
              </form>
            </CardContent>
          </Card>
        ) : (
          <Card className="border-emerald-900/50 bg-emerald-950/20 backdrop-blur mb-8">
            <CardHeader>
              <CardTitle className="text-emerald-400">Vault Unlocked</CardTitle>
              <CardDescription className="text-slate-400">
                Your vault is currently accessible
              </CardDescription>
            </CardHeader>
            <CardContent>
              <Button
                onClick={handleLockVault}
                variant="outline"
                className="border-slate-700 text-slate-300 hover:bg-slate-800 hover:text-white"
              >
                Lock Vault
              </Button>
            </CardContent>
          </Card>
        )}

        {isVaultUnlocked && token && <VaultFiles token={token} session={session} />}

        <div className="mt-6 grid grid-cols-1 md:grid-cols-2 gap-6">
          <Card className="border-slate-800 bg-slate-900/50 backdrop-blur">
            <CardHeader>
              <CardTitle className="text-white">Encryption Keys</CardTitle>
              <CardDescription className="text-slate-400">
                Manage your encryption keys
              </CardDescription>
            </CardHeader>
            <CardContent>
              <p className="text-slate-500 text-sm">Coming soon</p>
            </CardContent>
          </Card>

          <Card className="border-slate-800 bg-slate-900/50 backdrop-blur">
            <CardHeader>
              <CardTitle className="text-white">Access Logs</CardTitle>
              <CardDescription className="text-slate-400">
                View your security audit trail
              </CardDescription>
            </CardHeader>
            <CardContent>
              <p className="text-slate-500 text-sm">Coming soon</p>
            </CardContent>
          </Card>
        </div>

        <div className="mt-8">
          <Card className="border-slate-800 bg-slate-900/50 backdrop-blur">
            <CardHeader>
              <CardTitle className="text-white">Account Information</CardTitle>
            </CardHeader>
            <CardContent className="space-y-2">
              <div className="flex justify-between">
                <span className="text-slate-400">User ID:</span>
                <span className="text-slate-200 font-mono text-sm">{user?.id}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-slate-400">Email:</span>
                <span className="text-slate-200">{user?.email}</span>
              </div>
            </CardContent>
          </Card>
        </div>
      </main>
    </div>
  );
}
