'use client';

import { useEffect, useState } from 'react';
import { useRouter } from 'next/navigation';
import { useAuth } from '@/lib/auth-context';
import { api } from '@/lib/api';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';

export default function DashboardPage() {
  const router = useRouter();
  const { user, token, logout, isAuthenticated, isLoading } = useAuth();
  const [apiError, setApiError] = useState('');
  const [isVerifying, setIsVerifying] = useState(true);

  useEffect(() => {
    let isMounted = true;

    const verifyToken = async () => {
      if (!token) {
        return;
      }

      try {
        await api.getCurrentUser(token);
        if (isMounted) {
          setApiError('');
        }
      } catch {
        if (isMounted) {
          setApiError('Session expired. Please log in again.');
          setTimeout(() => {
            logout();
            router.push('/login');
          }, 2000);
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
    };
  }, [isAuthenticated, isLoading, token, router, logout]);

  const handleLogout = () => {
    logout();
    router.push('/login');
  };

  if (isLoading || isVerifying) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-gradient-to-br from-slate-950 via-slate-900 to-slate-950">
        <div className="text-slate-400">Loading...</div>
      </div>
    );
  }

  if (!isAuthenticated) {
    return null;
  }

  return (
    <div className="min-h-screen bg-gradient-to-br from-slate-950 via-slate-900 to-slate-950">
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
                className="border-slate-700 text-slate-300 hover:bg-slate-800 hover:text-white"
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

        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
          <Card className="border-slate-800 bg-slate-900/50 backdrop-blur">
            <CardHeader>
              <CardTitle className="text-white">Secure Files</CardTitle>
              <CardDescription className="text-slate-400">
                Upload and encrypt your sensitive files
              </CardDescription>
            </CardHeader>
            <CardContent>
              <p className="text-slate-500 text-sm">Coming soon</p>
            </CardContent>
          </Card>

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
