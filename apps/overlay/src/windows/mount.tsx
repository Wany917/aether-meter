import "../styles/base.css";
import "../ui/ui.css";

import { type ComponentType, type ReactNode, StrictMode, useEffect } from "react";
import { createRoot } from "react-dom/client";

import { commands } from "../lib/bindings";
import { loadGameFont } from "../lib/gameAssets";
import { SettingsProvider } from "../lib/settings";

function report(event: "ready" | "error", message: string): void {
  void commands.reportFrontend(event, message).catch(() => {
    // No IPC at all: nothing left to tell.
  });
}

/** Errors go to the app's log file: a release build has no console. */
function reportErrors(): void {
  window.addEventListener("error", (e) => report("error", `${e.message} (${e.filename}:${e.lineno})`));
  window.addEventListener("unhandledrejection", (e) => report("error", `unhandled rejection: ${String(e.reason)}`));
}

/** Tells the log the interface is on screen. */
function Ready({ children }: { children: ReactNode }) {
  useEffect(() => report("ready", ""), []);
  return children;
}

/** Every window: the game font, the settings, then its own view. */
export function mount(View: ComponentType): void {
  reportErrors();
  loadGameFont();
  const root = document.getElementById("root");
  if (!root) throw new Error("#root is missing from the page");
  createRoot(root).render(
    <StrictMode>
      <SettingsProvider>
        <Ready>
          <View />
        </Ready>
      </SettingsProvider>
    </StrictMode>,
  );
}
