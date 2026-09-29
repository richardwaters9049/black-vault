interface VaultMarkProps { size?: 'small' | 'large'; animated?: boolean }
export default function VaultMark({ size = 'small', animated = false }: VaultMarkProps) {
  return <span aria-hidden="true" className={`bv-mark ${size === 'large' ? 'bv-mark-large' : 'bv-mark-small'} ${animated ? 'bv-mark-animated' : ''}`}>
    <svg viewBox="0 0 120 120" fill="none" className="size-full" xmlns="http://www.w3.org/2000/svg">
      <path className="bv-mark-shell" d="M60 7 105 30v60L60 113 15 90V30L60 7Z" stroke="currentColor" strokeWidth="2.6" strokeLinejoin="round"/>
      <path className="bv-mark-top" d="M15 30 60 54l45-24M60 54v59" stroke="currentColor" strokeWidth="2" strokeLinejoin="round" opacity=".48"/>
      <path className="bv-mark-core" d="M60 23 88 38v43L60 96 32 81V38L60 23Z" stroke="currentColor" strokeWidth="2" strokeLinejoin="round"/>
      <path className="bv-mark-v" d="m45 46 15 30 15-30" stroke="currentColor" strokeWidth="5" strokeLinecap="round" strokeLinejoin="round"/>
      <circle className="bv-mark-dot" cx="60" cy="76" r="3.5" fill="currentColor"/>
      <path className="bv-mark-spark" d="M93 10v12m-6-6h12" stroke="#93c5fd" strokeWidth="2" strokeLinecap="round"/>
    </svg>
  </span>;
}
