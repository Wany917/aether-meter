import type { ClassId } from "../lib/bindings";
import { CLASS_MARKS } from "../lib/gameAssets";

/** The game's small class emblem, as next to names in rankings. */
export function ClassMark({ id, label, size = 20 }: { id: ClassId; label: string; size?: number }) {
  const src = CLASS_MARKS[id];
  if (!src) return <span className="class-mark class-mark-empty" style={{ width: size, height: size }} aria-label={label} />;
  return <img className="class-mark" src={src} alt={label} title={label} width={size} height={Math.round(size * 1.05)} draggable={false} />;
}
