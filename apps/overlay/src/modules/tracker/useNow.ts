import { useEffect, useState } from "react";

/** The clock, refreshed every `everyMs` while mounted. */
export function useNow(everyMs = 100): number {
  const [now, setNow] = useState(() => performance.now());
  useEffect(() => {
    const id = window.setInterval(() => setNow(performance.now()), everyMs);
    return () => window.clearInterval(id);
  }, [everyMs]);
  return now;
}
