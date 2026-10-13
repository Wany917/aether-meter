import type { EventCallback, UnlistenFn } from "@tauri-apps/api/event";
import { useEffect, useRef, useState } from "react";

import { type AppError, commands, events } from "./bindings";

interface EventSource<T> {
  listen: (callback: EventCallback<T>) => Promise<UnlistenFn>;
}

/** Run `handler` on every `source` event while the component is mounted. */
export function useEvent<T>(source: EventSource<T>, handler: (payload: T) => void): void {
  const latest = useRef(handler);
  latest.current = handler;

  useEffect(() => {
    let disposed = false;
    let unlisten: UnlistenFn | undefined;
    void source
      .listen((event) => latest.current(event.payload))
      .then((fn) => {
        if (disposed) fn();
        else unlisten = fn;
      });
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [source]);
}

/** A value seeded by a command, then kept fresh by the backend's pushes. */
function useLive<T>(load: () => Promise<T>, source: EventSource<T>): T | undefined {
  const [value, setValue] = useState<T>();
  useEvent(source, setValue);
  useEffect(() => {
    let alive = true;
    void load().then((initial) => {
      if (alive) setValue((current) => current ?? initial);
    });
    return () => {
      alive = false;
    };
  }, [load]);
  return value;
}

export const useMeter = () => useLive(commands.getMeter, events.meterUpdated);
export const useTracker = () => useLive(commands.getTracker, events.trackerUpdated);
export const useRoutine = () => useLive(commands.getRoutine, events.routineChanged);
export const usePets = () => useLive(commands.getPets, events.petsChanged);

type CommandResult<T> = { status: "ok"; data: T } | { status: "error"; error: AppError };

/** The data of a command result, or a thrown `CommandError`. */
export async function unwrap<T>(result: Promise<CommandResult<T>>): Promise<T> {
  const settled = await result;
  if (settled.status === "error") throw new CommandError(settled.error);
  return settled.data;
}

export class CommandError extends Error {
  constructor(readonly error: AppError) {
    super(error.message);
    this.name = `CommandError(${error.kind})`;
  }
}
