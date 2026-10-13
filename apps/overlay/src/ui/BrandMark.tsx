/** Aether's mark: the app icon, drawn small. */
export function BrandMark({ size = 26 }: { size?: number }) {
  return (
    <svg className="brand-mark" width={size} height={size} viewBox="0 0 1024 1024" aria-hidden="true">
      <defs>
        <linearGradient id="brand-gold" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" stopColor="#f6dc93" />
          <stop offset="0.55" stopColor="#d9b25e" />
          <stop offset="1" stopColor="#a87a2a" />
        </linearGradient>
      </defs>
      <path d="M512 36 L988 512 L512 988 L36 512 Z" fill="#111827" stroke="url(#brand-gold)" strokeWidth="44" strokeLinejoin="round" />
      <path d="M512 236 L728 742 L636 742 L512 438 L388 742 L296 742 Z" fill="url(#brand-gold)" />
      <path d="M512 572 L564 636 L512 700 L460 636 Z" fill="#e9fbff" />
    </svg>
  );
}
