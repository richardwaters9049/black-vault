'use client';

import { useState } from 'react';
import { useRouter } from 'next/navigation';
import Link from 'next/link';

import { useAuth } from '@/lib/auth-context';
import { api } from '@/lib/api';

import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';

export default function RegisterPage() {
  const router = useRouter();
  const { login } = useAuth();

  const [formData, setFormData] = useState({
    email: '',
    password: '',
    confirmPassword: '',
  });

  const [error, setError] = useState('');
  const [isLoading, setIsLoading] = useState(false);

  const validateForm = () => {
    if (
      !formData.email ||
      !formData.password ||
      !formData.confirmPassword
    ) {
      setError('All fields are required');
      return false;
    }

    const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;

    if (!emailRegex.test(formData.email)) {
      setError('Invalid email format');
      return false;
    }

    if (formData.password.length < 12) {
      setError('Password must be at least 12 characters');
      return false;
    }

    if (!/[A-Z]/.test(formData.password)) {
      setError('Password must contain at least one uppercase letter');
      return false;
    }

    if (!/[a-z]/.test(formData.password)) {
      setError('Password must contain at least one lowercase letter');
      return false;
    }

    if (!/[0-9]/.test(formData.password)) {
      setError('Password must contain at least one digit');
      return false;
    }

    if (formData.password !== formData.confirmPassword) {
      setError('Passwords do not match');
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
      await api.register({
        email: formData.email,
        password: formData.password,
      });

      const authResponse = await api.login({
        email: formData.email,
        password: formData.password,
      });

      login(authResponse.token, {
        id: authResponse.id,
        email: authResponse.email,
      });

      router.push('/dashboard');
    } catch (err) {
      setError(
        err instanceof Error
          ? err.message
          : 'Registration failed'
      );
    } finally {
      setIsLoading(false);
    }
  };

  const passwordRequirements = [
    {
      label: '12 characters minimum',
      valid: formData.password.length >= 12,
    },
    {
      label: 'One uppercase letter',
      valid: /[A-Z]/.test(formData.password),
    },
    {
      label: 'One lowercase letter',
      valid: /[a-z]/.test(formData.password),
    },
    {
      label: 'One number',
      valid: /[0-9]/.test(formData.password),
    },
  ];

  return (
    <main className="min-h-screen bg-[#050b1a] text-slate-100">
      {/* Background */}
      <div className="pointer-events-none fixed inset-0 overflow-hidden">
        <div className="absolute inset-0 bg-[radial-gradient(circle_at_top,rgba(59,130,246,0.08),transparent_40%)]" />
        <div className="absolute inset-0 bg-[linear-gradient(rgba(255,255,255,0.015)_1px,transparent_1px),linear-gradient(90deg,rgba(255,255,255,0.015)_1px,transparent_1px)] bg-[size:48px_48px]" />
      </div>

      {/* Header */}
      <header className="relative border-b border-white/[0.06]">
        <div className="mx-auto flex h-16 max-w-7xl items-center justify-between px-6 lg:px-8">
          <Link
            href="/"
            className="font-serif text-xl font-semibold tracking-tight text-white transition-opacity hover:opacity-80"
          >
            Black Vault
          </Link>

          <Link
            href="/login"
            className="text-sm text-slate-400 transition-colors hover:text-white"
          >
            Already have an account?
            <span className="ml-2 text-slate-200">
              Sign in
            </span>
          </Link>
        </div>
      </header>

      {/* Registration */}
      <section className="relative flex min-h-[calc(100vh-4rem)] items-center justify-center px-6 py-12">
        <div className="w-full max-w-md">

          {/* Security indicator */}
          <div className="mb-6 flex items-center justify-center gap-2 text-xs uppercase tracking-[0.2em] text-slate-500">
            <span className="h-1.5 w-1.5 rounded-full bg-emerald-400" />
            Secure account creation
          </div>

          {/* Card */}
          <div className="overflow-hidden rounded-2xl border border-white/[0.08] bg-[#0b1326]/90 shadow-2xl shadow-black/30 backdrop-blur-xl">

            {/* Card header */}
            <div className="border-b border-white/[0.06] px-7 py-7">
              <h1 className="font-serif text-3xl font-semibold tracking-tight text-white">
                Create your account
              </h1>

              <p className="mt-2 text-sm leading-6 text-slate-400">
                Create a secure account to begin protecting your files.
              </p>
            </div>

            {/* Form */}
            <form
              onSubmit={handleSubmit}
              className="space-y-6 px-7 py-7"
            >
              {/* Email */}
              <div className="space-y-2">
                <Label
                  htmlFor="email"
                  className="text-sm font-medium text-slate-200"
                >
                  Email address
                </Label>

                <Input
                  id="email"
                  type="email"
                  autoComplete="email"
                  placeholder="you@example.com"
                  value={formData.email}
                  onChange={(e) =>
                    setFormData({
                      ...formData,
                      email: e.target.value,
                    })
                  }
                  disabled={isLoading}
                  className="h-11 border-white/[0.1] bg-white/[0.03] text-white placeholder:text-slate-600 transition-colors focus:border-slate-400 focus:bg-white/[0.05] focus:ring-0"
                />
              </div>

              {/* Password */}
              <div className="space-y-2">
                <Label
                  htmlFor="password"
                  className="text-sm font-medium text-slate-200"
                >
                  Password
                </Label>

                <Input
                  id="password"
                  type="password"
                  autoComplete="new-password"
                  placeholder="Enter a secure password"
                  value={formData.password}
                  onChange={(e) =>
                    setFormData({
                      ...formData,
                      password: e.target.value,
                    })
                  }
                  disabled={isLoading}
                  className="h-11 border-white/[0.1] bg-white/[0.03] text-white placeholder:text-slate-600 transition-colors focus:border-slate-400 focus:bg-white/[0.05] focus:ring-0"
                />

                {/* Password requirements */}
                <div className="grid grid-cols-2 gap-x-4 gap-y-2 pt-2">
                  {passwordRequirements.map((requirement) => (
                    <div
                      key={requirement.label}
                      className="flex items-center gap-2 text-xs"
                    >
                      <span
                        className={`flex h-4 w-4 items-center justify-center rounded-full border text-[9px] ${requirement.valid
                          ? 'border-emerald-500/40 bg-emerald-500/10 text-emerald-400'
                          : 'border-white/10 text-slate-600'
                          }`}
                      >
                        {requirement.valid ? '✓' : ''}
                      </span>

                      <span
                        className={
                          requirement.valid
                            ? 'text-slate-300'
                            : 'text-slate-500'
                        }
                      >
                        {requirement.label}
                      </span>
                    </div>
                  ))}
                </div>
              </div>

              {/* Confirm password */}
              <div className="space-y-2">
                <Label
                  htmlFor="confirmPassword"
                  className="text-sm font-medium text-slate-200"
                >
                  Confirm password
                </Label>

                <Input
                  id="confirmPassword"
                  type="password"
                  autoComplete="new-password"
                  placeholder="Re-enter your password"
                  value={formData.confirmPassword}
                  onChange={(e) =>
                    setFormData({
                      ...formData,
                      confirmPassword: e.target.value,
                    })
                  }
                  disabled={isLoading}
                  className="h-11 border-white/[0.1] bg-white/[0.03] text-white placeholder:text-slate-600 transition-colors focus:border-slate-400 focus:bg-white/[0.05] focus:ring-0"
                />
              </div>

              {/* Error */}
              {error && (
                <div
                  role="alert"
                  className="rounded-lg border border-red-500/20 bg-red-500/[0.06] px-4 py-3 text-sm text-red-300"
                >
                  {error}
                </div>
              )}

              {/* Submit */}
              <Button
                type="submit"
                disabled={isLoading}
                className="h-11 w-full bg-slate-100 font-medium text-slate-950 transition-all hover:bg-white hover:shadow-lg hover:shadow-white/5 disabled:opacity-50"
              >
                {isLoading
                  ? 'Creating account...'
                  : 'Create account'}
              </Button>
            </form>

            {/* Footer */}
            <div className="border-t border-white/[0.06] px-7 py-5">
              <p className="text-center text-xs leading-5 text-slate-500">
                By creating an account, you agree to use Black Vault
                responsibly and keep your credentials secure.
              </p>

              <p className="mt-4 text-center text-sm text-slate-500">
                Already have an account?{' '}
                <Link
                  href="/login"
                  className="text-slate-200 underline decoration-slate-600 underline-offset-4 transition-colors hover:text-white hover:decoration-slate-300"
                >
                  Sign in
                </Link>
              </p>
            </div>
          </div>

          {/* Trust statement */}
          <p className="mt-6 text-center text-xs text-slate-600">
            Your credentials are protected using Argon2 password hashing.
          </p>
        </div>
      </section>
    </main>
  );
}