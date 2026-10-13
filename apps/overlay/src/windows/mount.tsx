import "../styles/base.css";
import "../ui/ui.css";

import { type ComponentType, StrictMode } from "react";
import { createRoot } from "react-dom/client";

import { loadGameFont } from "../lib/gameAssets";
import { SettingsProvider } from "../lib/settings";

/** Every window: the game font, the settings, then its own view. */
export function mount(View: ComponentType): void {
  loadGameFont();
  const root = document.getElementById("root");
  if (!root) throw new Error("#root is missing from the page");
  createRoot(root).render(
    <StrictMode>
      <SettingsProvider>
        <View />
      </SettingsProvider>
    </StrictMode>,
  );
}
