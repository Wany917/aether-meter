import { useEffect, useRef, useState } from "react";

const reducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)");

/**
 * `target`, eased over `durationMs` each time it changes. Snapshots land every
 * 300 ms; easing over about that long turns the steps into continuous motion.
 */
export function useTweened(target: number, durationMs = 300): number {
  const [value, setValue] = useState(target);
  const shown = useRef(target);

  useEffect(() => {
    if (reducedMotion.matches || !Number.isFinite(target)) {
      shown.current = target;
      setValue(target);
      return;
    }
    const from = shown.current;
    const start = performance.now();
    let frame = 0;
    const step = (now: number) => {
      const t = Math.min(1, (now - start) / durationMs);
      const eased = 1 - (1 - t) ** 3;
      shown.current = from + (target - from) * eased;
      setValue(shown.current);
      if (t < 1) frame = requestAnimationFrame(step);
    };
    frame = requestAnimationFrame(step);
    return () => cancelAnimationFrame(frame);
  }, [target, durationMs]);

  return value;
}
