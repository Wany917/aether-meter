import { createContext, type ReactNode, useCallback, useContext, useEffect, useMemo, useState } from "react";

import { commands, events, type Settings } from "./bindings";
import { type Translate, translator } from "./i18n";
import { unwrap, useEvent } from "./ipc";

interface SettingsContextValue {
  settings: Settings;
  t: Translate;
  /** Persist new settings; rejects with a `CommandError` the UI can show. */
  save: (next: Settings) => Promise<Settings>;
}

const SettingsContext = createContext<SettingsContextValue | null>(null);

/** Loads the settings once per window and follows every change after. */
export function SettingsProvider({ children }: { children: ReactNode }) {
  const [settings, setSettings] = useState<Settings>();
  useEvent(events.settingsChanged, setSettings);

  useEffect(() => {
    void commands.getSettings().then((initial) => setSettings((current) => current ?? initial));
  }, []);

  useEffect(() => {
    if (!settings) return;
    const root = document.documentElement;
    root.lang = settings.language;
    root.style.setProperty("--ui-scale", String(settings.meter.scale));
    root.style.setProperty("--panel-alpha", String(settings.meter.opacity));
    root.dataset.clickThrough = String(settings.clickThrough);
  }, [settings]);

  const save = useCallback(async (next: Settings) => {
    const saved = await unwrap(commands.saveSettings(next));
    setSettings(saved);
    return saved;
  }, []);

  const value = useMemo(
    () => (settings ? { settings, t: translator(settings.language), save } : null),
    [settings, save],
  );
  return value ? <SettingsContext.Provider value={value}>{children}</SettingsContext.Provider> : null;
}

export function useSettings(): SettingsContextValue {
  const value = useContext(SettingsContext);
  if (!value) throw new Error("useSettings outside <SettingsProvider>");
  return value;
}
