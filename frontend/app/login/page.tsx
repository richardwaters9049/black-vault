'use client';

import { useEffect, useState } from 'react';
import { useRouter } from 'next/navigation';
import Link from 'next/link';
import { useAuth } from '@/lib/auth-context';
import { api } from '@/lib/api';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';

export default function LoginPage() {
  const router = useRouter();
  const auth = useAuth();

  const [formData, setFormData] = useState({
    email: '',
    password: '',
  });

  const [error, setError] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  const [showPassword, setShowPassword] = useState(false);

  useEffect(() => {
    if (auth.isAuthenticated) {
      router.replace('/dashboard');
    }
  }, [auth.isAuthenticated, router]);

  const validateForm = () => {
    if (!formData.email || !formData.password) {
      setError('Email and password are required');
      return false;
    }

    const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;

    if (!emailRegex.test(formData.email)) {
      setError('Invalid email format');
      return false;
    }

    return true;
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError('');

    if (!validateForm()) {
      return;
    }

    setIsLoading(true);

    try {
      const authResponse = await api.login({
        email: formData.email,
        password: formData.password,
      });

      auth.login(authResponse.token, {
        id: authResponse.id,
        email: authResponse.email,
      });

      router.push('/dashboard');
    } catch (err) {
      setError(
        err instanceof Error
          ? err.message
          : 'Unable to sign in. Please check your credentials.',
      );
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <main className="relative min-h-screen overflow-hidden bg-[#05070a]">
      <div className="absolute inset-0 bg-[radial-gradient(circle_at_50%_20%,rgba(37,99,235,0.08),transparent_38%)]" />

      <div className="absolute inset-0 bg-[linear-gradient(rgba(255,255,255,0.025)_1px,transparent_1px),linear-gradient(90deg,rgba(255,255,255,0.025)_1px,transparent_1px)] bg-[size:48px_48px]" />

      <div className="absolute inset-x-0 bottom-0 h-1/2 bg-gradient-to-t from-[#05070a] to-transparent" />

      <div className="relative z-10 grid min-h-screen grid-cols-1 items-center px-5 py-10 sm:px-8 sm:py-12 lg:px-12">
        <div className="mx-auto grid w-full max-w-lg grid-cols-1">
          <header className="mb-10 grid justify-items-center sm:mb-12">
            <div className="flex items-center justify-center gap-4">
              <Link
                href="/"
                aria-label="Return to Black Vault home"
                className="group flex h-12 w-12 cursor-pointer items-center justify-center rounded-lg border border-blue-400/20 bg-blue-500/[0.06] transition-colors duration-200 hover:border-blue-400/35 hover:bg-blue-500/[0.10]"
              >
                <span className="text-2xl font-semibold tracking-[0.08em] text-blue-400">
                  BV
                </span>
              </Link>

              <Link
                href="/"
                aria-label="Return to Black Vault home"
                className="cursor-pointer text-lg font-medium tracking-[0.32em] text-slate-300 transition-colors duration-200 hover:text-white sm:text-4xl"
              >
                BLACK VAULT
              </Link>
            </div>
          </header>

          <section className="grid overflow-hidden rounded-2xl border border-white/[0.08] bg-[#090c11]/95 shadow-2xl shadow-black/50 backdrop-blur-xl">
            <div className="h-px w-full bg-gradient-to-r from-transparent via-blue-500/60 to-transparent" />

            <div className="grid gap-10 p-8 sm:p-10 md:p-12">
              <header className="grid gap-5">
                <div className="flex items-center gap-3">
                  <span className="h-2 w-2 rounded-full bg-blue-400 shadow-[0_0_10px_rgba(96,165,250,0.7)]" />

                  <span className="text-xs font-medium uppercase tracking-[0.24em] text-slate-600">
                    Secure Authentication
                  </span>
                </div>

                <div className="grid gap-3">
                  <h1 className="text-4xl font-medium tracking-[-0.02em] text-white sm:text-5xl">
                    Sign in
                  </h1>

                  <p className="max-w-md text-base leading-7 tracking-[0.015em] text-slate-500 sm:text-lg sm:leading-8">
                    Access your Black Vault secure encryption environment.
                  </p>
                </div>
              </header>

              <form
                onSubmit={handleSubmit}
                className="grid gap-6"
              >
                <div className="grid gap-2.5">
                  <Label
                    htmlFor="email"
                    className="text-sm font-medium tracking-[0.04em] text-slate-400 sm:text-base"
                  >
                    Email
                  </Label>

                  <Input
                    id="email"
                    name="email"
                    type="email"
                    autoComplete="email"
                    placeholder="you@example.com"
                    value={formData.email}
                    onChange={(e) => {
                      setFormData({
                        ...formData,
                        email: e.target.value,
                      });

                      if (error) {
                        setError('');
                      }
                    }}
                    disabled={isLoading}
                    className="h-13 cursor-text border-white/[0.08] bg-white/[0.025] px-4 text-base tracking-[0.015em] text-white placeholder:text-slate-700 transition-colors duration-200 focus:border-blue-500/40 focus:ring-1 focus:ring-blue-500/20 sm:h-14 sm:text-lg"
                  />
                </div>

                <div className="grid gap-2.5">
                  <Label
                    htmlFor="password"
                    className="text-sm font-medium tracking-[0.04em] text-slate-400 sm:text-base"
                  >
                    Password
                  </Label>

                  <div className="relative grid">
                    <Input
                      id="password"
                      name="password"
                      type={showPassword ? 'text' : 'password'}
                      autoComplete="current-password"
                      placeholder="Enter your password"
                      value={formData.password}
                      onChange={(e) => {
                        setFormData({
                          ...formData,
                          password: e.target.value,
                        });

                        if (error) {
                          setError('');
                        }
                      }}
                      disabled={isLoading}
                      className="h-13 cursor-text border-white/[0.08] bg-white/[0.025] px-4 pr-24 text-base tracking-[0.015em] text-white placeholder:text-slate-700 transition-colors duration-200 focus:border-blue-500/40 focus:ring-1 focus:ring-blue-500/20 sm:h-14 sm:text-lg"
                    />

                    <button
                      type="button"
                      onClick={() => setShowPassword((value) => !value)}
                      disabled={isLoading}
                      aria-label={
                        showPassword
                          ? 'Hide password'
                          : 'Show password'
                      }
                      className="absolute right-4 top-1/2 -translate-y-1/2 cursor-pointer text-xs font-medium uppercase tracking-[0.16em] text-slate-600 transition-colors duration-200 hover:text-blue-400 disabled:cursor-not-allowed disabled:opacity-50"
                    >
                      {showPassword ? 'Hide' : 'Show'}
                    </button>
                  </div>
                </div>

                {error && (
                  <div
                    role="alert"
                    className="grid rounded-lg border border-red-500/15 bg-red-500/[0.05] px-4 py-3.5 text-sm leading-6 tracking-[0.01em] text-red-400 sm:text-base"
                  >
                    {error}
                  </div>
                )}

                <Button
                  type="submit"
                  disabled={isLoading}
                  className="h-13 w-full cursor-pointer border border-blue-400/20 bg-blue-500/[0.10] text-base font-medium tracking-[0.06em] text-blue-300 transition-all duration-200 hover:border-blue-400/30 hover:bg-blue-500/[0.16] hover:text-blue-200 disabled:cursor-not-allowed disabled:opacity-50 sm:h-14 sm:text-lg"
                >
                  {isLoading ? (
                    <span className="flex items-center justify-center gap-3">
                      <span className="h-4 w-4 animate-spin rounded-full border-2 border-blue-300/20 border-t-blue-300" />

                      <span className="tracking-[0.04em]">
                        Signing in...
                      </span>
                    </span>
                  ) : (
                    'Sign in'
                  )}
                </Button>
              </form>

              <div className="grid gap-6">
                <div className="h-px bg-white/[0.06]" />

                <div className="text-center">
                  <p className="text-sm leading-6 tracking-[0.01em] text-slate-600 sm:text-base">
                    Don&apos;t have an account?{' '}
                    <Link
                      href="/register"
                      className="cursor-pointer font-medium tracking-[0.02em] text-slate-400 transition-colors duration-200 hover:text-blue-400"
                    >
                      Create account
                    </Link>
                  </p>
                </div>
              </div>
            </div>
          </section>

          <footer className="mt-7 grid">
            <div className="flex items-center justify-center gap-3">
              <span className="h-1.5 w-1.5 rounded-full bg-emerald-500/60" />

              <span className="text-[11px] uppercase tracking-[0.22em] text-slate-700 sm:text-xs">
                Secure encrypted session
              </span>
            </div>
          </footer>
        </div>
      </div>
    </main>
  );
}