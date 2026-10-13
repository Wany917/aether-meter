import type { SVGProps } from "react";

/** Thin-stroke glyphs drawn for this UI, in the weight of the game's own. */
const paths = {
  reset: "M3.5 8a4.5 4.5 0 1 0 1.32-3.18M3.5 3v2.5H6",
  details: "M3 4.5h10M3 8h10M3 11.5h6",
  settings:
    "M8 5.6a2.4 2.4 0 1 0 0 4.8 2.4 2.4 0 0 0 0-4.8ZM8 1.8v1.6M8 12.6v1.6M3.6 3.6l1.13 1.13M11.27 11.27l1.13 1.13M1.8 8h1.6M12.6 8h1.6M3.6 12.4l1.13-1.13M11.27 4.73l1.13-1.13",
  lock: "M4.5 7V5.2a3.5 3.5 0 0 1 7 0V7M3.5 7h9v6.5h-9z",
  unlock: "M4.5 7V5.2a3.5 3.5 0 0 1 6.6-1.6M3.5 7h9v6.5h-9z",
  close: "M3.5 3.5l9 9M12.5 3.5l-9 9",
  minimize: "M3.5 8h9",
  plus: "M8 3.5v9M3.5 8h9",
  hide: "M2 8s2.2-4 6-4 6 4 6 4-2.2 4-6 4-6-4-6-4ZM3 13 13 3",
  trash: "M3 4.5h10M6.5 4.5V3h3v1.5M4.5 4.5l.6 9h5.8l.6-9",
  back: "M9.5 3.5 5 8l4.5 4.5",
  buffs: "M8 2.5 13 8l-5 5.5L3 8z M8 5.5 10.5 8 8 10.5 5.5 8z",
  swords: "M2.5 2.5l7 7M9.5 9.5l1.5 1.5M11 11l2 2M13.5 2.5l-7 7M6.5 9.5 5 11M5 11l-2 2M2.5 13.5l1-1M13.5 13.5l-1-1",
} as const;

export type IconName = keyof typeof paths;

export function Icon({ name, ...props }: { name: IconName } & SVGProps<SVGSVGElement>) {
  return (
    <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth={1.3} strokeLinecap="round" strokeLinejoin="round" aria-hidden="true" {...props}>
      <path d={paths[name]} />
    </svg>
  );
}
