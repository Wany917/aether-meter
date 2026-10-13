import { useState } from "react";

import { iconSource } from "../lib/gameAssets";
import { Icon } from "./Icon";

/** URLs that already failed this session: never asked for twice. */
const missing = new Set<string>();

/**
 * A skill or buff icon: tries each candidate URL in order and falls back to a
 * neutral glyph (basic attacks have no icon of their own).
 */
export function GameIcon({ candidates, label, size }: { candidates: string[]; label: string; size: number }) {
  const [, rerender] = useState(0);
  const url = candidates.map(iconSource).find((u) => !missing.has(u));

  return (
    <span className="game-icon" style={{ width: size, height: size }}>
      {url ? (
        <img
          src={url}
          alt={label}
          width={size}
          height={size}
          draggable={false}
          onError={() => {
            missing.add(url);
            rerender((n) => n + 1);
          }}
        />
      ) : (
        <Icon name="swords" className="game-icon-fallback" />
      )}
    </span>
  );
}
