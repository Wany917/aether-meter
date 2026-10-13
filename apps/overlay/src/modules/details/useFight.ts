import { useEffect, useState } from "react";

import { commands, type FightSource, type FightView } from "../../lib/bindings";
import { CommandError, unwrap } from "../../lib/ipc";

const LIVE_REFRESH_MS = 1000;

export interface FightState {
  /** `undefined` while loading, `null` when there is no live fight. */
  fight: FightView | null | undefined;
  error: string | null;
}

/** The live fight, refreshed every second, or one saved fight. */
export function useFight(source: FightSource, active: boolean): FightState {
  const [state, setState] = useState<FightState>({ fight: undefined, error: null });
  const savedId = source.kind === "saved" ? source.id : null;

  // A new source starts from blank; coming back to the same one keeps showing
  // the last fight until the refresh lands.
  useEffect(() => {
    setState({ fight: undefined, error: null });
  }, [savedId]);

  useEffect(() => {
    if (!active) return;
    let alive = true;

    if (savedId !== null) {
      unwrap(commands.getFight(savedId))
        .then((fight) => alive && setState({ fight, error: null }))
        .catch((e: unknown) => alive && setState({ fight: null, error: e instanceof CommandError ? e.message : String(e) }));
      return () => {
        alive = false;
      };
    }

    const refresh = () =>
      void commands.getLiveFight().then((fight) => {
        if (alive) setState({ fight, error: null });
      });
    refresh();
    const id = window.setInterval(refresh, LIVE_REFRESH_MS);
    return () => {
      alive = false;
      window.clearInterval(id);
    };
  }, [savedId, active]);

  return state;
}
