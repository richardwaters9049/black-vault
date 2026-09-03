'use client';

import Link from 'next/link';
import { motion } from 'framer-motion';
import {
  ArrowRight,
  Check,
  FileLock2,
  LockKeyhole,
} from 'lucide-react';

import { Button } from '@/components/ui/button';

const securityPoints = [
  'End-to-end encryption',
  'JWT authenticated access',
  'Argon2 password hashing',
];

const vaultFiles = [
  {
    name: 'financial-records.pdf',
    size: '2.4 MB',
  },
  {
    name: 'identity-documents.zip',
    size: '8.7 MB',
  },
  {
    name: 'private-project.tar',
    size: '14.2 MB',
  },
];

export default function HomePage() {
  return (
    <main className="relative overflow-hidden bg-[#05070a] text-slate-100">
      {/* Background atmosphere */}
      <div
        aria-hidden="true"
        className="pointer-events-none absolute inset-0"
      >
        <div className="absolute inset-0 bg-[radial-gradient(circle_at_50%_0%,rgba(37,99,235,0.10),transparent_42%)]" />

        <div className="absolute inset-0 bg-[linear-gradient(rgba(255,255,255,0.025)_1px,transparent_1px),linear-gradient(90deg,rgba(255,255,255,0.025)_1px,transparent_1px)] bg-[size:48px_48px]" />

        <div className="absolute inset-x-0 top-0 h-96 bg-gradient-to-b from-[#05070a]/0 via-[#05070a]/20 to-[#05070a]" />
      </div>

      <div className="relative z-10 grid grid-cols-1">
        {/* Header */}
        <header className="px-5 pt-6 sm:px-8 sm:pt-8 lg:px-12">
          <motion.div
            initial={{ opacity: 0, y: -12 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{
              duration: 0.6,
              ease: [0.22, 1, 0.36, 1],
            }}
            className="mx-auto flex w-full max-w-7xl items-center justify-between"
          >
            <Link
              href="/"
              aria-label="Return to Black Vault home"
              className="group flex items-center gap-3"
            >
              <div className="flex h-10 w-10 items-center justify-center rounded-lg border border-blue-400/20 bg-blue-500/[0.06] transition-colors duration-200 group-hover:border-blue-400/35 group-hover:bg-blue-500/[0.10]">
                <span className="text-sm font-semibold tracking-[0.08em] text-blue-400">
                  BV
                </span>
              </div>

              <div className="grid gap-0.5">
                <span className="text-sm font-medium tracking-[0.22em] text-slate-300 transition-colors duration-200 group-hover:text-white sm:text-base">
                  BLACK VAULT
                </span>

                <span className="hidden text-[9px] uppercase tracking-[0.22em] text-slate-700 sm:block">
                  Secure file infrastructure
                </span>
              </div>
            </Link>

            <nav className="flex items-center gap-2 sm:gap-4">
              <Link
                href="/login"
                className="rounded-md px-3 py-2 text-sm font-medium tracking-[0.02em] text-slate-500 transition-colors duration-200 hover:text-white sm:px-4"
              >
                Sign in
              </Link>

              <Link href="/register">
                <Button
                  size="sm"
                  className="h-10 border border-blue-400/20 bg-blue-500/[0.10] px-4 text-sm font-medium text-blue-300 shadow-none transition-all duration-200 hover:border-blue-400/30 hover:bg-blue-500/[0.16] hover:text-blue-200"
                >
                  Get started
                </Button>
              </Link>
            </nav>
          </motion.div>
        </header>

        {/* Main hero */}
        <section className="mx-auto grid w-full max-w-7xl grid-cols-1 px-5 pb-16 pt-16 sm:px-8 sm:pb-20 sm:pt-20 lg:grid-cols-[1fr_0.9fr] lg:items-center lg:gap-20 lg:px-12 lg:pb-28 lg:pt-28">
          {/* Hero content */}
          <motion.div
            initial={{ opacity: 0, x: -24 }}
            animate={{ opacity: 1, x: 0 }}
            transition={{
              duration: 0.7,
              delay: 0.1,
              ease: [0.22, 1, 0.36, 1],
            }}
            className="grid"
          >
            <div className="grid gap-3">
              <div className="flex items-center gap-3">
                <span className="h-2 w-2 rounded-full bg-blue-400 shadow-[0_0_12px_rgba(96,165,250,0.8)]" />

                <span className="text-xs font-medium uppercase tracking-[0.24em] text-blue-400/80 sm:text-sm">
                  Security-first file storage
                </span>
              </div>

              <div className="grid gap-5 pt-3">
                <h1 className="max-w-3xl text-5xl font-medium leading-[1.02] tracking-[-0.045em] text-white sm:text-6xl lg:text-7xl xl:text-[5.25rem]">
                  Your files.
                  <br />
                  <span className="text-slate-500">
                    Locked down.
                  </span>
                </h1>

                <p className="max-w-2xl text-lg leading-8 tracking-[0.01em] text-slate-400 sm:text-xl sm:leading-9">
                  Black Vault is a secure environment for encrypting,
                  storing and transferring sensitive files while keeping
                  control of your data where it belongs.
                </p>
              </div>
            </div>

            <div className="mt-9 flex flex-col gap-3 sm:flex-row">
              <Link href="/register">
                <Button
                  size="lg"
                  className="h-14 w-full border border-blue-400/20 bg-blue-500/[0.12] px-7 text-base font-medium tracking-[0.03em] text-blue-200 shadow-[0_0_35px_rgba(37,99,235,0.08)] transition-all duration-200 hover:border-blue-400/35 hover:bg-blue-500/[0.18] hover:text-white sm:w-auto"
                >
                  Create your vault
                  <ArrowRight className="ml-2 h-4 w-4" />
                </Button>
              </Link>

              <Link href="/login">
                <Button
                  size="lg"
                  variant="ghost"
                  className="h-14 w-full border border-white/[0.08] bg-white/[0.025] px-7 text-base font-medium tracking-[0.03em] text-slate-300 transition-all duration-200 hover:border-white/[0.12] hover:bg-white/[0.05] hover:text-white sm:w-auto"
                >
                  Sign in
                </Button>
              </Link>
            </div>

            <div className="mt-8 flex flex-wrap gap-x-6 gap-y-3">
              {securityPoints.map((point, index) => (
                <motion.div
                  key={point}
                  initial={{ opacity: 0, y: 8 }}
                  animate={{ opacity: 1, y: 0 }}
                  transition={{
                    duration: 0.45,
                    delay: 0.45 + index * 0.1,
                    ease: [0.22, 1, 0.36, 1],
                  }}
                  className="flex items-center gap-2 text-sm text-slate-500"
                >
                  <Check className="h-4 w-4 text-emerald-400/80" />

                  <span>{point}</span>
                </motion.div>
              ))}
            </div>
          </motion.div>

          {/* Vault visual */}
          <motion.div
            initial={{ opacity: 0, x: 28, scale: 0.97 }}
            animate={{ opacity: 1, x: 0, scale: 1 }}
            transition={{
              duration: 0.8,
              delay: 0.25,
              ease: [0.22, 1, 0.36, 1],
            }}
            className="relative mt-14 lg:mt-0"
          >
            <div
              aria-hidden="true"
              className="pointer-events-none absolute -inset-16 rounded-full bg-blue-500/[0.055] blur-3xl"
            />

            <motion.div
              animate={{
                y: [0, -6, 0],
              }}
              transition={{
                duration: 7,
                repeat: Infinity,
                ease: 'easeInOut',
              }}
              className="relative overflow-hidden rounded-2xl border border-white/[0.09] bg-[#090c11]/95 shadow-2xl shadow-black/60 backdrop-blur-xl"
            >
              {/* Vault header */}
              <div className="flex items-center justify-between border-b border-white/[0.07] px-5 py-4 sm:px-6">
                <div className="flex items-center gap-3">
                  <span className="h-2 w-2 rounded-full bg-emerald-400 shadow-[0_0_10px_rgba(52,211,153,0.5)]" />

                  <span className="text-sm font-medium tracking-[0.02em] text-slate-300">
                    Secure Vault
                  </span>
                </div>

                <span className="font-mono text-[10px] tracking-[0.16em] text-slate-700">
                  ENCRYPTED
                </span>
              </div>

              {/* Vault body */}
              <div className="grid gap-8 p-6 sm:p-8">
                <div className="flex items-start justify-between gap-6">
                  <div className="grid gap-2">
                    <span className="text-xs font-medium uppercase tracking-[0.2em] text-slate-600">
                      Private storage
                    </span>

                    <span className="text-2xl font-medium tracking-[-0.025em] text-white sm:text-3xl">
                      Your Vault
                    </span>
                  </div>

                  <div className="flex h-12 w-12 shrink-0 items-center justify-center rounded-xl border border-blue-400/15 bg-blue-500/[0.07]">
                    <LockKeyhole className="h-5 w-5 text-blue-300" />
                  </div>
                </div>

                <div className="grid gap-3">
                  {vaultFiles.map((file, index) => (
                    <motion.div
                      key={file.name}
                      initial={{ opacity: 0, x: 12 }}
                      animate={{ opacity: 1, x: 0 }}
                      transition={{
                        duration: 0.45,
                        delay: 0.65 + index * 0.12,
                        ease: [0.22, 1, 0.36, 1],
                      }}
                    >
                      <VaultFile
                        name={file.name}
                        size={file.size}
                      />
                    </motion.div>
                  ))}
                </div>

                <div className="grid gap-3 border-t border-white/[0.07] pt-6">
                  <div className="flex items-center justify-between">
                    <span className="text-sm text-slate-500">
                      Vault status
                    </span>

                    <span className="flex items-center gap-2 text-sm font-medium text-emerald-400">
                      <span className="h-1.5 w-1.5 rounded-full bg-emerald-400" />
                      Protected
                    </span>
                  </div>

                  <div className="h-1 overflow-hidden rounded-full bg-white/[0.04]">
                    <motion.div
                      initial={{ width: 0 }}
                      animate={{ width: '100%' }}
                      transition={{
                        duration: 1.2,
                        delay: 0.8,
                        ease: [0.22, 1, 0.36, 1],
                      }}
                      className="h-full rounded-full bg-blue-500/50"
                    />
                  </div>
                </div>
              </div>
            </motion.div>
          </motion.div>
        </section>

        {/* Bottom identity strip */}
        <motion.section
          initial={{ opacity: 0, y: 12 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{
            duration: 0.6,
            delay: 0.8,
            ease: [0.22, 1, 0.36, 1],
          }}
          className="border-t border-white/[0.06] px-5 py-8 sm:px-8 lg:px-12"
        >
          <div className="mx-auto flex w-full max-w-7xl flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
            <div className="flex items-center gap-3">
              <div className="flex h-8 w-8 items-center justify-center rounded-md border border-blue-400/15 bg-blue-500/[0.05]">
                <span className="text-[10px] font-semibold tracking-[0.08em] text-blue-400">
                  BV
                </span>
              </div>

              <span className="text-xs font-medium uppercase tracking-[0.18em] text-slate-600">
                Black Vault
              </span>
            </div>

            <span className="text-xs tracking-[0.02em] text-slate-700">
              Secure file infrastructure
            </span>
          </div>
        </motion.section>
      </div>
    </main>
  );
}

function VaultFile({
  name,
  size,
}: {
  name: string;
  size: string;
}) {
  return (
    <div className="flex items-center justify-between gap-4 rounded-xl border border-white/[0.06] bg-white/[0.025] px-4 py-4 transition-colors duration-200 hover:border-white/[0.09] hover:bg-white/[0.04]">
      <div className="flex min-w-0 items-center gap-3">
        <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-lg border border-white/[0.05] bg-white/[0.025]">
          <FileLock2 className="h-4 w-4 text-slate-500" />
        </div>

        <div className="grid min-w-0 gap-1">
          <p className="truncate text-sm font-medium tracking-[0.01em] text-slate-300">
            {name}
          </p>

          <p className="font-mono text-[10px] tracking-[0.04em] text-slate-600">
            {size}
          </p>
        </div>
      </div>

      <span className="shrink-0 text-[10px] font-medium uppercase tracking-[0.16em] text-emerald-400/60">
        Locked
      </span>
    </div>
  );
}