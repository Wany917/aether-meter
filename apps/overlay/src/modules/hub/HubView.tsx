import { getCurrentWindow } from "@tauri-apps/api/window";
import { useState } from "react";

import { events, type FightSource, type HubTab } from "../../lib/bindings";
import { useEvent } from "../../lib/ipc";
import { useSettings } from "../../lib/settings";
import { BrandMark } from "../../ui/BrandMark";
import { Icon } from "../../ui/Icon";
import { Tabs } from "../../ui/Tabs";
import { DetailsTab } from "../details/DetailsTab";
import { HistoryTab } from "../history/HistoryTab";
import { RoutineTab } from "../routine/RoutineTab";
import { SettingsTab } from "../settings/SettingsTab";
import { useDocumentVisible } from "./useDocumentVisible";

const LIVE: FightSource = { kind: "live" };

/** Details, history and settings, in the layout of the game's own windows. */
export function HubView() {
  const { t } = useSettings();
  const visible = useDocumentVisible();
  const [tab, setTab] = useState<HubTab>("details");
  const [source, setSource] = useState<FightSource>(LIVE);
  const [focusActor, setFocusActor] = useState<number | null>(null);

  useEvent(events.hubNavigate, ({ tab: next, actorId }) => {
    setTab(next);
    if (next === "details" && actorId !== null) {
      setSource(LIVE);
      setFocusActor(actorId);
    }
  });

  const openSaved = (id: string) => {
    setSource({ kind: "saved", id });
    setFocusActor(null);
    setTab("details");
  };

  const window = getCurrentWindow();

  return (
    <div className="hub">
      <header className="hub-bar" data-tauri-drag-region>
        <div className="hub-brand" data-tauri-drag-region>
          <BrandMark size={22} />
          <span className="hub-title" data-tauri-drag-region>
            Aether
          </span>
        </div>
        <Tabs
          tabs={[
            { id: "details", label: t("tab.details") },
            { id: "history", label: t("tab.history") },
            { id: "routine", label: t("tab.routine") },
            { id: "settings", label: t("tab.settings") },
          ]}
          active={tab}
          onSelect={setTab}
        />
        <div className="hub-window-controls">
          <button type="button" className="icon-btn" title={t("hub.minimize")} onClick={() => void window.minimize()}>
            <Icon name="minimize" />
          </button>
          <button type="button" className="icon-btn hub-close" title={t("hub.close")} onClick={() => void window.hide()}>
            <Icon name="close" />
          </button>
        </div>
      </header>
      <main className="hub-content">
        {tab === "details" && (
          <DetailsTab source={source} focusActor={focusActor} active={visible} onBackToLive={() => setSource(LIVE)} />
        )}
        {tab === "history" && <HistoryTab active={visible} onOpen={openSaved} />}
        {tab === "routine" && <RoutineTab />}
        {tab === "settings" && <SettingsTab />}
      </main>
    </div>
  );
}
