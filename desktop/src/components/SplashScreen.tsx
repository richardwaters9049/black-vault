import { LockKeyhole } from 'lucide-react';

export default function SplashScreen() {
  return <div className="vault-splash fixed inset-0 grid place-items-center px-6" role="status" aria-label="Initialising Black Vault">
    <div className="flex flex-col items-center text-center">
      <div className="vault-emblem relative mb-10 grid size-24 place-items-center">
        <span className="vault-ring absolute inset-2 rotate-45 rounded-2xl border border-indigo-400/60" />
        <span className="vault-ring vault-ring-inner absolute inset-5 rotate-45 rounded-xl border border-blue-300/70" />
        <LockKeyhole size={26} strokeWidth={1.3} className="vault-lock relative text-indigo-200" />
      </div>
      <h1 className="vault-reveal text-2xl font-semibold tracking-[0.26em]">BLACK VAULT</h1>
      <p className="vault-reveal mt-4 font-mono text-[10px] uppercase tracking-[0.23em] text-slate-500">Your private space, offline.</p>
      <div className="mt-9 h-px w-44 overflow-hidden bg-slate-800"><div className="vault-progress h-full bg-indigo-400" /></div>
    </div>
  </div>;
}
