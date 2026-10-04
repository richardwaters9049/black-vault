import { ShieldCheck } from 'lucide-react';
import VaultMark from './VaultMark';

export default function VaultHero(): React.ReactElement {
  return (
    <div
      className="bv-vault-hero relative mx-auto aspect-square w-full max-w-[430px]"
      aria-label="Futuristic Black Vault security door"
    >
      <div className="bv-vault-ambient absolute inset-[4%] rounded-full" aria-hidden="true" />
      <div className="bv-vault-grid absolute inset-[9%]" aria-hidden="true" />

      <div className="bv-bank-vault absolute left-1/2 top-1/2" aria-hidden="true">
        <div className="bv-bank-vault-frame absolute inset-0" />
        <div className="bv-bank-vault-ring bv-bank-vault-ring-outer absolute inset-[6%]" />
        <div className="bv-bank-vault-ring bv-bank-vault-ring-mid absolute inset-[13%]" />
        <div className="bv-bank-vault-ring bv-bank-vault-ring-inner absolute inset-[20%]" />

        <div className="bv-bank-vault-door absolute inset-[24%]">
          <div className="bv-bank-vault-door-face absolute inset-0" />
          <div className="bv-bank-vault-door-rim absolute inset-[8%]" />

          <span className="bv-bank-bolt bv-bank-bolt-1" />
          <span className="bv-bank-bolt bv-bank-bolt-2" />
          <span className="bv-bank-bolt bv-bank-bolt-3" />
          <span className="bv-bank-bolt bv-bank-bolt-4" />
          <span className="bv-bank-bolt bv-bank-bolt-5" />
          <span className="bv-bank-bolt bv-bank-bolt-6" />
          <span className="bv-bank-bolt bv-bank-bolt-7" />
          <span className="bv-bank-bolt bv-bank-bolt-8" />

          <div className="bv-bank-lock-wheel absolute left-1/2 top-1/2">
            <span className="bv-bank-spoke bv-bank-spoke-1" />
            <span className="bv-bank-spoke bv-bank-spoke-2" />
            <span className="bv-bank-spoke bv-bank-spoke-3" />
            <span className="bv-bank-spoke bv-bank-spoke-4" />
            <span className="bv-bank-spoke bv-bank-spoke-5" />
            <span className="bv-bank-spoke bv-bank-spoke-6" />

            <div className="bv-bank-brand-core absolute left-1/2 top-1/2">
              <div className="bv-bank-brand-halo absolute inset-[-18%]" />
              <VaultMark size="large" />
            </div>
          </div>
        </div>

        <div className="bv-bank-security-sweep absolute inset-[16%]" />
        <div className="bv-bank-light bv-bank-light-left" />
        <div className="bv-bank-light bv-bank-light-right" />
      </div>

      <div className="bv-vault-status absolute left-[7%] top-[17%]">
        <ShieldCheck size={13} />
        <span>AES-256 protected</span>
      </div>

      <div className="bv-vault-status absolute bottom-[19%] right-[4%]">
        <span className="bv-vault-status-dot" />
        <span>Local secure storage</span>
      </div>

      <div className="absolute inset-x-0 bottom-4 text-center">
        <span className="rounded-full border border-blue-400/15 bg-[#08111d]/85 px-3 py-1.5 text-[11px] tracking-wide text-blue-200 backdrop-blur-sm">
          Your Black Vault is ready to be created
        </span>
      </div>
    </div>
  );
}
