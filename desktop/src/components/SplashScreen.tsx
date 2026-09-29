import VaultMark from './VaultMark';

export default function SplashScreen() {
  return <div className="vault-splash fixed inset-0 grid place-items-center overflow-hidden px-6" role="status" aria-label="Initialising Black Vault">
    <div className="vault-splash-grid pointer-events-none absolute inset-0" aria-hidden="true" />
    <div className="relative flex flex-col items-center text-center">
      <div className="bv-logo-stage relative mb-8 grid size-40 place-items-center">
        <span aria-hidden="true" className="bv-orbit bv-orbit-one absolute inset-1 rounded-full border border-blue-400/15" />
        <span aria-hidden="true" className="bv-orbit bv-orbit-two absolute inset-4 rounded-full border border-dashed border-sky-300/20" />
        <span aria-hidden="true" className="bv-orbit-blip absolute left-1/2 top-0 size-1.5 rounded-full bg-sky-300 shadow-[0_0_14px_#38bdf8]" />
        <VaultMark size="large" animated />
      </div>
      <h1 className="vault-reveal text-[25px] font-semibold tracking-[0.28em] text-slate-100">BLACK <span className="text-sky-400">VAULT</span></h1>
      <p className="vault-reveal mt-3 font-mono text-[10px] uppercase tracking-[0.23em] text-slate-400">Your private space, offline.</p>
      <div className="mt-9 h-[2px] w-40 overflow-hidden rounded-full bg-slate-800"><div className="vault-progress h-full rounded-full bg-gradient-to-r from-blue-600 via-sky-400 to-blue-300" /></div>
      <span className="vault-reveal mt-3 font-mono text-[9px] uppercase tracking-[0.2em] text-slate-600">Securing your workspace</span>
    </div>
  </div>;
}
