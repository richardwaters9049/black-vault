import VaultMark from './VaultMark';

export default function SplashScreen(): React.ReactElement {
  return (
    <div
      className="vault-splash fixed inset-0 z-50 grid place-items-center overflow-hidden px-6"
      role="status"
      aria-label="Initialising Black Vault"
    >
      <div className="vault-splash-grid pointer-events-none absolute inset-0" aria-hidden="true" />

      <div className="bv-logo-stage relative grid size-44 place-items-center">
        <span aria-hidden="true" className="bv-orbit bv-orbit-one absolute inset-1 rounded-full border border-blue-400/15" />
        <span aria-hidden="true" className="bv-orbit bv-orbit-two absolute inset-4 rounded-full border border-dashed border-sky-300/20" />
        <span aria-hidden="true" className="bv-orbit-blip absolute left-1/2 top-0 size-1.5 rounded-full bg-sky-300 shadow-[0_0_14px_#38bdf8]" />
        <VaultMark size="large" animated />
      </div>

      <div className="bv-logo-copy absolute top-[calc(50%+7rem)] text-center">
        <h1 className="text-[25px] font-semibold tracking-[0.28em] text-slate-100">
          BLACK <span className="text-sky-400">VAULT</span>
        </h1>
        <p className="mt-3 font-mono text-[10px] uppercase tracking-[0.22em] text-slate-500">
          Secure local encryption
        </p>
      </div>
    </div>
  );
}
