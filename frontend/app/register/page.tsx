'use client';

import { useState } from 'react';
import { motion, type Variants } from 'framer-motion';
import { useRouter } from 'next/navigation';
import Link from 'next/link';
import { Eye, EyeOff } from 'lucide-react';

import { useAuth } from '@/lib/auth-context';
import { api } from '@/lib/api';
import { generateVaultKey, encryptVaultKey } from '@/lib/crypto';

import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';

const pageVariants: Variants = {
  hidden: {
    opacity: 0,
  },
  visible: {
    opacity: 1,
    transition: {
      duration: 0.45,
      ease: 'easeOut',
      staggerChildren: 0.08,
    },
  },
};

const itemVariants: Variants = {
  hidden: {
    opacity: 0,
    y: 18,
  },
  visible: {
    opacity: 1,
    y: 0,
    transition: {
      duration: 0.45,
      ease: 'easeOut',
    },
  },
};

const cardVariants: Variants = {
  hidden: {
    opacity: 0,
    y: 28,
    scale: 0.985,
  },
  visible: {
    opacity: 1,
    y: 0,
    scale: 1,
    transition: {
      duration: 0.6,
      ease: 'easeOut',
    },
  },
};

export default function RegisterPage() {
  const router = useRouter();
  const { login } = useAuth();

  const [formData, setFormData] = useState({
    email: '',
    password: '',
    confirmPassword: '',
    vaultPassword: '',
    confirmVaultPassword: '',
  });

  const [error, setError] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  const [showVaultPassword, setShowVaultPassword] = useState(false);
  const [showConfirmVaultPassword, setShowConfirmVaultPassword] =
    useState(false);

  const validateForm = () => {
    if (
      !formData.email ||
      !formData.password ||
      !formData.confirmPassword ||
      !formData.vaultPassword ||
      !formData.confirmVaultPassword
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

    if (formData.vaultPassword.length < 12) {
      setError('Vault password must be at least 12 characters');
      return false;
    }

    if (!/[A-Z]/.test(formData.vaultPassword)) {
      setError('Vault password must contain at least one uppercase letter');
      return false;
    }

    if (!/[a-z]/.test(formData.vaultPassword)) {
      setError('Vault password must contain at least one lowercase letter');
      return false;
    }

    if (!/[0-9]/.test(formData.vaultPassword)) {
      setError('Vault password must contain at least one digit');
      return false;
    }

    if (formData.vaultPassword !== formData.confirmVaultPassword) {
      setError('Vault passwords do not match');
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

      const vaultKey = generateVaultKey();

      const vaultEnvelope = await encryptVaultKey(
        vaultKey,
        formData.vaultPassword,
      );

      await api.setupVault(authResponse.token, {
        encrypted_vault_key: vaultEnvelope.encrypted_vault_key,
        key_derivation_salt: vaultEnvelope.key_derivation_salt,
        key_derivation_params: vaultEnvelope.key_derivation_params,
        encryption_nonce: vaultEnvelope.encryption_nonce,
      });

      router.push('/dashboard');
    } catch (err) {
      setError(
        err instanceof Error
          ? err.message
          : 'Registration failed',
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

  const vaultPasswordRequirements = [
    {
      label: '12 characters minimum',
      valid: formData.vaultPassword.length >= 12,
    },
    {
      label: 'One uppercase letter',
      valid: /[A-Z]/.test(formData.vaultPassword),
    },
    {
      label: 'One lowercase letter',
      valid: /[a-z]/.test(formData.vaultPassword),
    },
    {
      label: 'One number',
      valid: /[0-9]/.test(formData.vaultPassword),
    },
  ];

  return (
    <main className="relative overflow-hidden bg-[#050b1a] text-slate-100">
      {/* Background */}
      <div className="pointer-events-none fixed inset-0 overflow-hidden">
        <div className="absolute inset-0 bg-[radial-gradient(circle_at_top,rgba(59,130,246,0.10),transparent_42%)]" />

        <div className="absolute inset-0 bg-[linear-gradient(rgba(255,255,255,0.018)_1px,transparent_1px),linear-gradient(90deg,rgba(255,255,255,0.018)_1px,transparent_1px)] bg-[size:48px_48px]" />

        <motion.div
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          transition={{
            duration: 1.2,
            ease: 'easeOut',
          }}
          className="absolute left-1/2 top-[-18rem] h-[36rem] w-[36rem] -translate-x-1/2 rounded-full bg-blue-500/[0.07] blur-3xl"
        />

        <motion.div
          initial={{
            opacity: 0,
            scale: 0.8,
          }}
          animate={{
            opacity: 1,
            scale: 1,
          }}
          transition={{
            duration: 1.4,
            delay: 0.2,
            ease: 'easeOut',
          }}
          className="absolute bottom-[-18rem] right-[-12rem] h-[34rem] w-[34rem] rounded-full bg-indigo-500/[0.05] blur-3xl"
        />
      </div>

      {/* Header */}
      <motion.header
        initial={{
          opacity: 0,
          y: -12,
        }}
        animate={{
          opacity: 1,
          y: 0,
        }}
        transition={{
          duration: 0.5,
          ease: 'easeOut',
        }}
        className="relative border-b border-white/[0.06]"
      >
        <div className="mx-auto flex w-full max-w-7xl items-center justify-between gap-6 px-5 py-5 sm:px-8 lg:px-12">
          <Link
            href="/"
            className="font-serif text-xl font-semibold tracking-tight text-white transition-opacity duration-200 hover:opacity-80"
          >
            Black Vault
          </Link>

          <Link
            href="/login"
            className="text-right text-sm text-slate-400 transition-colors duration-200 hover:text-white"
          >
            <span className="hidden sm:inline">
              Already have an account?
            </span>

            <span className="sm:ml-2">
              Sign in
            </span>
          </Link>
        </div>
      </motion.header>

      {/* Registration */}
      <motion.section
        variants={pageVariants}
        initial="hidden"
        animate="visible"
        className="relative grid justify-items-center px-5 py-12 sm:px-8 sm:py-16 lg:px-12 lg:py-20"
      >
        <div className="grid w-full max-w-md gap-6">
          {/* Security indicator */}
          <motion.div
            variants={itemVariants}
            className="flex items-center justify-center gap-3 text-xs uppercase tracking-[0.2em] text-slate-500"
          >
            <span className="relative flex h-2 w-2">
              <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-emerald-400 opacity-30" />

              <span className="relative inline-flex h-2 w-2 rounded-full bg-emerald-400" />
            </span>

            Secure account creation
          </motion.div>

          {/* Card */}
          <motion.div
            variants={cardVariants}
            className="overflow-hidden rounded-2xl border border-white/[0.08] bg-[#0b1326]/95 shadow-2xl shadow-black/40 backdrop-blur-xl"
          >
            {/* Card accent */}
            <div className="h-px w-full bg-gradient-to-r from-transparent via-blue-500/70 to-transparent" />

            {/* Card header */}
            <div className="border-b border-white/[0.06] px-7 py-8 sm:px-8 sm:py-9">
              <div className="grid gap-4">
                <div className="flex items-center gap-3">
                  <span className="h-2 w-2 rounded-full bg-blue-400 shadow-[0_0_12px_rgba(96,165,250,0.7)]" />

                  <span className="text-xs font-medium uppercase tracking-[0.22em] text-slate-600">
                    Create your vault
                  </span>
                </div>

                <div className="grid gap-3">
                  <h1 className="font-serif text-3xl font-semibold tracking-tight text-white sm:text-4xl">
                    Create your account
                  </h1>

                  <p className="text-sm leading-6 text-slate-400 sm:text-base sm:leading-7">
                    Set up your account and create the encryption credentials
                    that protect your vault.
                  </p>
                </div>
              </div>
            </div>

            {/* Form */}
            <form
              onSubmit={handleSubmit}
              className="grid gap-7 px-7 py-8 sm:px-8 sm:py-9"
            >
              {/* Email */}
              <div className="grid gap-2.5">
                <Label
                  htmlFor="email"
                  className="text-sm font-medium tracking-[0.03em] text-slate-200"
                >
                  Email address
                </Label>

                <Input
                  id="email"
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
                  className="h-12 border-white/[0.1] bg-white/[0.03] px-4 text-base text-white placeholder:text-slate-600 transition-colors duration-200 focus:border-blue-500/40 focus:bg-white/[0.045] focus:ring-1 focus:ring-blue-500/20"
                />
              </div>

              {/* Account password */}
              <div className="grid gap-2.5">
                <Label
                  htmlFor="password"
                  className="text-sm font-medium tracking-[0.03em] text-slate-200"
                >
                  Password
                </Label>

                <Input
                  id="password"
                  type="password"
                  autoComplete="new-password"
                  placeholder="Enter a secure password"
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
                  className="h-12 border-white/[0.1] bg-white/[0.03] px-4 text-base text-white placeholder:text-slate-600 transition-colors duration-200 focus:border-blue-500/40 focus:bg-white/[0.045] focus:ring-1 focus:ring-blue-500/20"
                />

                {/* Password requirements */}
                <div className="grid grid-cols-1 gap-2 pt-2 sm:grid-cols-2">
                  {passwordRequirements.map((requirement) => (
                    <div
                      key={requirement.label}
                      className="flex items-center gap-2 text-xs"
                    >
                      <span
                        className={`flex h-4 w-4 shrink-0 items-center justify-center rounded-full border text-[9px] transition-colors duration-200 ${requirement.valid
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

              {/* Confirm account password */}
              <div className="grid gap-2.5">
                <Label
                  htmlFor="confirmPassword"
                  className="text-sm font-medium tracking-[0.03em] text-slate-200"
                >
                  Confirm password
                </Label>

                <Input
                  id="confirmPassword"
                  type="password"
                  autoComplete="new-password"
                  placeholder="Re-enter your password"
                  value={formData.confirmPassword}
                  onChange={(e) => {
                    setFormData({
                      ...formData,
                      confirmPassword: e.target.value,
                    });

                    if (error) {
                      setError('');
                    }
                  }}
                  disabled={isLoading}
                  className="h-12 border-white/[0.1] bg-white/[0.03] px-4 text-base text-white placeholder:text-slate-600 transition-colors duration-200 focus:border-blue-500/40 focus:bg-white/[0.045] focus:ring-1 focus:ring-blue-500/20"
                />
              </div>

              {/* Vault password */}
              <div className="grid gap-2.5">
                <div className="grid gap-1">
                  <Label
                    htmlFor="vaultPassword"
                    className="text-sm font-medium tracking-[0.03em] text-slate-200"
                  >
                    Vault password
                  </Label>

                  <p className="text-xs leading-5 text-slate-600">
                    Used to protect the encryption key for your vault.
                  </p>
                </div>

                <div className="relative">
                  <Input
                    id="vaultPassword"
                    type={showVaultPassword ? 'text' : 'password'}
                    autoComplete="new-password"
                    placeholder="Create a vault password"
                    value={formData.vaultPassword}
                    onChange={(e) => {
                      setFormData({
                        ...formData,
                        vaultPassword: e.target.value,
                      });

                      if (error) {
                        setError('');
                      }
                    }}
                    disabled={isLoading}
                    className="h-12 border-white/[0.1] bg-white/[0.03] px-4 pr-14 text-base text-white placeholder:text-slate-600 transition-colors duration-200 focus:border-blue-500/40 focus:bg-white/[0.045] focus:ring-1 focus:ring-blue-500/20"
                  />

                  <button
                    type="button"
                    onClick={() =>
                      setShowVaultPassword((value) => !value)
                    }
                    disabled={isLoading}
                    aria-label={
                      showVaultPassword
                        ? 'Hide vault password'
                        : 'Show vault password'
                    }
                    className="absolute right-3 top-1/2 flex h-9 w-9 -translate-y-1/2 cursor-pointer items-center justify-center rounded-md text-slate-600 transition-colors duration-200 hover:bg-white/[0.04] hover:text-blue-400 disabled:cursor-not-allowed disabled:opacity-50"
                  >
                    {showVaultPassword ? (
                      <EyeOff className="h-4 w-4" />
                    ) : (
                      <Eye className="h-4 w-4" />
                    )}
                  </button>
                </div>

                {/* Vault password requirements */}
                <div className="grid grid-cols-1 gap-2 pt-2 sm:grid-cols-2">
                  {vaultPasswordRequirements.map((requirement) => (
                    <div
                      key={requirement.label}
                      className="flex items-center gap-2 text-xs"
                    >
                      <span
                        className={`flex h-4 w-4 shrink-0 items-center justify-center rounded-full border text-[9px] transition-colors duration-200 ${requirement.valid
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

              {/* Confirm vault password */}
              <div className="grid gap-2.5">
                <Label
                  htmlFor="confirmVaultPassword"
                  className="text-sm font-medium tracking-[0.03em] text-slate-200"
                >
                  Confirm vault password
                </Label>

                <div className="relative">
                  <Input
                    id="confirmVaultPassword"
                    type={
                      showConfirmVaultPassword
                        ? 'text'
                        : 'password'
                    }
                    autoComplete="new-password"
                    placeholder="Re-enter your vault password"
                    value={formData.confirmVaultPassword}
                    onChange={(e) => {
                      setFormData({
                        ...formData,
                        confirmVaultPassword: e.target.value,
                      });

                      if (error) {
                        setError('');
                      }
                    }}
                    disabled={isLoading}
                    className="h-12 border-white/[0.1] bg-white/[0.03] px-4 pr-14 text-base text-white placeholder:text-slate-600 transition-colors duration-200 focus:border-blue-500/40 focus:bg-white/[0.045] focus:ring-1 focus:ring-blue-500/20"
                  />

                  <button
                    type="button"
                    onClick={() =>
                      setShowConfirmVaultPassword(
                        (value) => !value,
                      )
                    }
                    disabled={isLoading}
                    aria-label={
                      showConfirmVaultPassword
                        ? 'Hide confirmed vault password'
                        : 'Show confirmed vault password'
                    }
                    className="absolute right-3 top-1/2 flex h-9 w-9 -translate-y-1/2 cursor-pointer items-center justify-center rounded-md text-slate-600 transition-colors duration-200 hover:bg-white/[0.04] hover:text-blue-400 disabled:cursor-not-allowed disabled:opacity-50"
                  >
                    {showConfirmVaultPassword ? (
                      <EyeOff className="h-4 w-4" />
                    ) : (
                      <Eye className="h-4 w-4" />
                    )}
                  </button>
                </div>
              </div>

              {/* Error */}
              {error && (
                <motion.div
                  initial={{
                    opacity: 0,
                    y: -6,
                  }}
                  animate={{
                    opacity: 1,
                    y: 0,
                  }}
                  transition={{
                    duration: 0.25,
                    ease: 'easeOut',
                  }}
                  role="alert"
                  className="grid rounded-lg border border-red-500/20 bg-red-500/[0.06] px-4 py-3.5 text-sm leading-6 text-red-300"
                >
                  {error}
                </motion.div>
              )}

              {/* Submit */}
              <Button
                type="submit"
                disabled={isLoading}
                className="h-12 w-full cursor-pointer border border-blue-400/20 bg-blue-500/[0.10] text-base font-medium tracking-[0.04em] text-blue-200 transition-all duration-200 hover:border-blue-400/30 hover:bg-blue-500/[0.16] hover:text-blue-100 hover:shadow-lg hover:shadow-blue-500/[0.06] disabled:cursor-not-allowed disabled:opacity-50"
              >
                {isLoading ? (
                  <span className="flex items-center justify-center gap-3">
                    <span className="h-4 w-4 animate-spin rounded-full border-2 border-blue-300/20 border-t-blue-300" />

                    Creating account...
                  </span>
                ) : (
                  'Create account'
                )}
              </Button>
            </form>

            {/* Footer */}
            <div className="grid gap-5 border-t border-white/[0.06] px-7 py-6 sm:px-8">
              <p className="text-center text-xs leading-5 text-slate-600">
                By creating an account, you agree to use Black Vault
                responsibly and keep your credentials secure.
              </p>

              <p className="text-center text-sm text-slate-500">
                Already have an account?{' '}
                <Link
                  href="/login"
                  className="font-medium text-slate-300 underline decoration-slate-700 underline-offset-4 transition-colors duration-200 hover:text-blue-400 hover:decoration-blue-400/50"
                >
                  Sign in
                </Link>
              </p>
            </div>
          </motion.div>

          {/* Trust statement */}
          <motion.div
            variants={itemVariants}
            className="flex items-center justify-center gap-3 text-center"
          >
            <span className="h-1.5 w-1.5 rounded-full bg-emerald-500/60" />

            <p className="text-xs tracking-[0.04em] text-slate-600">
              Credentials protected using Argon2 password hashing.
            </p>
          </motion.div>
        </div>
      </motion.section>
    </main>
  );
}