import Link from 'next/link';

import {
  ArrowRight,
  Check,
  FileLock2,
  KeyRound,
  LockKeyhole,
  ShieldCheck,
} from 'lucide-react';

import { Button } from '@/components/ui/button';

const features = [
  {
    icon: FileLock2,
    title: 'Encrypted before upload',
    description:
      'Files are protected before they leave your device, keeping sensitive data private throughout the transfer.',
  },
  {
    icon: KeyRound,
    title: 'Keys stay under your control',
    description:
      'Encryption keys are separated from your stored files and never exposed through the application interface.',
  },
  {
    icon: ShieldCheck,
    title: 'Security-first architecture',
    description:
      'Designed around least privilege, strong authentication and secure handling of sensitive information.',
  },
];

const securityPoints = [
  'End-to-end encryption',
  'JWT authenticated access',
  'Argon2 password hashing',
];

export default function HomePage() {
  return (
    <div className="min-h-screen overflow-hidden bg-[#050914] text-slate-100">
      {/* Background */}
      <div className="pointer-events-none fixed inset-0 -z-10">
        <div
          className="absolute inset-0 opacity-[0.035]"
          style={{
            backgroundImage:
              'linear-gradient(rgba(148,163,184,0.8) 1px, transparent 1px), linear-gradient(90deg, rgba(148,163,184,0.8) 1px, transparent 1px)',
            backgroundSize: '48px 48px',
          }}
        />

        <div className="absolute left-1/2 top-[-20rem] h-[42rem] w-[42rem] -translate-x-1/2 rounded-full bg-blue-500/[0.07] blur-3xl" />

        <div className="absolute bottom-[-20rem] right-[-10rem] h-[36rem] w-[36rem] rounded-full bg-indigo-500/[0.05] blur-3xl" />
      </div>

      {/* Header */}
      <header className="border-b border-white/[0.06] bg-[#050914]/80 backdrop-blur-xl">
        <div className="mx-auto flex h-20 max-w-7xl items-center justify-between px-6 lg:px-8">
          <Link
            href="/"
            className="group flex items-center gap-3"
            aria-label="Black Vault home"
          >
            <div className="flex h-9 w-9 items-center justify-center rounded-lg border border-white/10 bg-white/[0.04] transition-colors group-hover:border-white/20">
              <LockKeyhole className="h-4 w-4 text-slate-200" />
            </div>

            <div>
              <span className="block text-sm font-semibold tracking-tight text-white">
                Black Vault
              </span>
              <span className="hidden text-[10px] uppercase tracking-[0.2em] text-slate-500 sm:block">
                Secure file infrastructure
              </span>
            </div>
          </Link>

          <nav className="flex items-center gap-3">
            <Link
              href="/login"
              className="rounded-md px-4 py-2 text-sm text-slate-400 transition-colors hover:text-white"
            >
              Sign in
            </Link>

            <Link href="/register">
              <Button
                size="sm"
                className="bg-slate-100 text-slate-950 shadow-none hover:bg-white"
              >
                Get started
              </Button>
            </Link>
          </nav>
        </div>
      </header>

      <main>
        {/* Hero */}
        <section className="mx-auto max-w-7xl px-6 pb-24 pt-20 lg:px-8 lg:pb-32 lg:pt-28">
          <div className="grid items-center gap-16 lg:grid-cols-[1.05fr_0.95fr] lg:gap-20">
            {/* Hero copy */}
            <div>
              <div className="mb-7 inline-flex items-center gap-2 rounded-full border border-emerald-400/10 bg-emerald-400/[0.04] px-3 py-1.5">
                <span className="relative flex h-2 w-2">
                  <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-emerald-400 opacity-40" />
                  <span className="relative inline-flex h-2 w-2 rounded-full bg-emerald-400" />
                </span>

                <span className="text-[11px] font-medium uppercase tracking-[0.16em] text-emerald-300/80">
                  Security-first file storage
                </span>
              </div>

              <h1 className="max-w-3xl text-5xl font-semibold leading-[1.02] tracking-[-0.04em] text-white sm:text-6xl lg:text-7xl">
                Your files.
                <br />
                <span className="text-slate-400">Locked down.</span>
              </h1>

              <p className="mt-7 max-w-xl text-base leading-7 text-slate-400 sm:text-lg">
                Black Vault is a secure platform for encrypting, storing and
                transferring sensitive files without compromising control over
                your data.
              </p>

              <div className="mt-9 flex flex-col gap-3 sm:flex-row">
                <Link href="/register">
                  <Button
                    size="lg"
                    className="h-12 w-full bg-slate-100 px-6 text-slate-950 shadow-[0_0_30px_rgba(255,255,255,0.06)] hover:bg-white sm:w-auto"
                  >
                    Create your vault
                    <ArrowRight className="ml-2 h-4 w-4" />
                  </Button>
                </Link>

                <Link href="/login">
                  <Button
                    size="lg"
                    variant="ghost"
                    className="h-12 w-full border border-white/[0.08] bg-white/[0.02] px-6 text-slate-300 hover:bg-white/[0.06] hover:text-white sm:w-auto"
                  >
                    Sign in
                  </Button>
                </Link>
              </div>

              <div className="mt-8 flex flex-wrap gap-x-6 gap-y-3">
                {securityPoints.map((point) => (
                  <div
                    key={point}
                    className="flex items-center gap-2 text-xs text-slate-500"
                  >
                    <Check className="h-3.5 w-3.5 text-emerald-400/80" />
                    {point}
                  </div>
                ))}
              </div>
            </div>

            {/* Vault visual */}
            <div className="relative">
              <div className="absolute -inset-10 rounded-full bg-blue-500/[0.05] blur-3xl" />

              <div className="relative overflow-hidden rounded-2xl border border-white/[0.08] bg-[#0a1020]/90 shadow-2xl shadow-black/30">
                {/* Window header */}
                <div className="flex items-center justify-between border-b border-white/[0.06] px-5 py-4">
                  <div className="flex items-center gap-2">
                    <div className="h-2 w-2 rounded-full bg-emerald-400" />
                    <span className="text-xs font-medium text-slate-300">
                      Secure Vault
                    </span>
                  </div>

                  <span className="font-mono text-[10px] text-slate-600">
                    ENCRYPTED
                  </span>
                </div>

                {/* Vault content */}
                <div className="p-6 sm:p-8">
                  <div className="mb-8 flex items-center justify-between">
                    <div>
                      <p className="text-xs uppercase tracking-[0.16em] text-slate-600">
                        Private storage
                      </p>
                      <p className="mt-2 text-2xl font-semibold tracking-tight text-white">
                        Your Vault
                      </p>
                    </div>

                    <div className="flex h-12 w-12 items-center justify-center rounded-xl border border-blue-400/10 bg-blue-400/[0.05]">
                      <LockKeyhole className="h-5 w-5 text-blue-300" />
                    </div>
                  </div>

                  <div className="space-y-3">
                    <VaultFile
                      name="financial-records.pdf"
                      size="2.4 MB"
                    />

                    <VaultFile
                      name="identity-documents.zip"
                      size="8.7 MB"
                    />

                    <VaultFile
                      name="private-project.tar"
                      size="14.2 MB"
                    />
                  </div>

                  <div className="mt-7 border-t border-white/[0.06] pt-5">
                    <div className="flex items-center justify-between">
                      <span className="text-xs text-slate-500">
                        Vault status
                      </span>

                      <span className="flex items-center gap-2 text-xs text-emerald-400">
                        <span className="h-1.5 w-1.5 rounded-full bg-emerald-400" />
                        Protected
                      </span>
                    </div>
                  </div>
                </div>
              </div>

              {/* Floating security badge */}
              <div className="absolute -bottom-5 -left-5 hidden items-center gap-3 rounded-xl border border-white/[0.08] bg-[#0b1222] px-4 py-3 shadow-xl sm:flex">
                <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-emerald-400/[0.08]">
                  <ShieldCheck className="h-4 w-4 text-emerald-400" />
                </div>

                <div>
                  <p className="text-[10px] uppercase tracking-wider text-slate-600">
                    Protection
                  </p>
                  <p className="mt-0.5 text-xs font-medium text-slate-300">
                    Encryption active
                  </p>
                </div>
              </div>
            </div>
          </div>
        </section>

        {/* Feature section */}
        <section className="border-y border-white/[0.06] bg-white/[0.015]">
          <div className="mx-auto max-w-7xl px-6 py-20 lg:px-8 lg:py-24">
            <div className="mb-12 max-w-2xl">
              <p className="text-xs font-medium uppercase tracking-[0.2em] text-blue-400/80">
                Built for sensitive data
              </p>

              <h2 className="mt-3 text-3xl font-semibold tracking-[-0.025em] text-white sm:text-4xl">
                Security is part of the architecture.
              </h2>

              <p className="mt-4 text-sm leading-6 text-slate-500 sm:text-base">
                Black Vault is being built around the principle that sensitive
                files should remain protected throughout their entire lifecycle.
              </p>
            </div>

            <div className="grid gap-px overflow-hidden rounded-2xl border border-white/[0.07] bg-white/[0.07] md:grid-cols-3">
              {features.map((feature) => {
                const Icon = feature.icon;

                return (
                  <div
                    key={feature.title}
                    className="bg-[#080e1b] p-7 transition-colors hover:bg-[#0b1222] lg:p-8"
                  >
                    <div className="flex h-10 w-10 items-center justify-center rounded-lg border border-white/[0.07] bg-white/[0.03]">
                      <Icon className="h-4 w-4 text-slate-300" />
                    </div>

                    <h3 className="mt-6 text-sm font-semibold text-white">
                      {feature.title}
                    </h3>

                    <p className="mt-3 text-sm leading-6 text-slate-500">
                      {feature.description}
                    </p>
                  </div>
                );
              })}
            </div>
          </div>
        </section>

        {/* Final CTA */}
        <section className="mx-auto max-w-7xl px-6 py-24 lg:px-8 lg:py-32">
          <div className="relative overflow-hidden rounded-2xl border border-white/[0.08] bg-[#0a1020] px-6 py-14 text-center sm:px-12">
            <div className="pointer-events-none absolute left-1/2 top-0 h-48 w-96 -translate-x-1/2 rounded-full bg-blue-500/[0.06] blur-3xl" />

            <div className="relative">
              <LockKeyhole className="mx-auto h-6 w-6 text-slate-400" />

              <h2 className="mx-auto mt-5 max-w-xl text-3xl font-semibold tracking-[-0.03em] text-white sm:text-4xl">
                Keep sensitive files under your control.
              </h2>

              <p className="mx-auto mt-4 max-w-lg text-sm leading-6 text-slate-500">
                Create your secure vault and start building a safer way to
                handle sensitive information.
              </p>

              <div className="mt-8">
                <Link href="/register">
                  <Button
                    size="lg"
                    className="bg-slate-100 px-6 text-slate-950 hover:bg-white"
                  >
                    Create your vault
                    <ArrowRight className="ml-2 h-4 w-4" />
                  </Button>
                </Link>
              </div>
            </div>
          </div>
        </section>
      </main>

      {/* Footer */}
      <footer className="border-t border-white/[0.06]">
        <div className="mx-auto flex max-w-7xl flex-col gap-4 px-6 py-8 sm:flex-row sm:items-center sm:justify-between lg:px-8">
          <div className="flex items-center gap-2">
            <LockKeyhole className="h-3.5 w-3.5 text-slate-600" />
            <span className="text-xs text-slate-600">
              Black Vault
            </span>
          </div>

          <p className="text-xs text-slate-700">
            Secure file infrastructure.
          </p>
        </div>
      </footer>
    </div>
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
    <div className="flex items-center justify-between rounded-xl border border-white/[0.05] bg-white/[0.02] px-4 py-3">
      <div className="flex min-w-0 items-center gap-3">
        <div className="flex h-8 w-8 shrink-0 items-center justify-center rounded-lg bg-white/[0.04]">
          <FileLock2 className="h-3.5 w-3.5 text-slate-500" />
        </div>

        <div className="min-w-0">
          <p className="truncate text-xs font-medium text-slate-300">
            {name}
          </p>

          <p className="mt-0.5 font-mono text-[10px] text-slate-600">
            {size}
          </p>
        </div>
      </div>

      <span className="ml-4 shrink-0 text-[9px] font-medium uppercase tracking-wider text-emerald-400/60">
        Locked
      </span>
    </div>
  );
}