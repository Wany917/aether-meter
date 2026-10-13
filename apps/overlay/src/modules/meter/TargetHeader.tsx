import { getCurrentWindow } from "@tauri-apps/api/window";

import { commands, type MeterSnapshot } from "../../lib/bindings";
import { formatDuration, formatPercent } from "../../lib/format";
import { targetModeName } from "../../lib/i18n";
import { useSettings } from "../../lib/settings";
import { Icon } from "../../ui/Icon";

/** Drag handle, target and timer, HP, and the window's few controls. */
export function TargetHeader({ meter }: { meter: MeterSnapshot | undefined }) {
  const { settings, t, save } = useSettings();
  const target = meter?.target ?? null;
  const fighting = meter?.players.some((p) => p.damage > 0) ?? false;
  // Modes that follow no single target name the mode instead.
  const untargeted =
    fighting && settings.targetMode === "allTargets" ? targetModeName("allTargets", settings.language) : t("meter.noTarget");
  const hp = target?.hpRatio ?? null;

  return (
    <header className="meter-header" data-tauri-drag-region>
      <div className="meter-title" data-tauri-drag-region>
        <span className="target-mark" data-boss={target?.isBoss || undefined} aria-hidden="true" />
        <span className="target-name" data-tauri-drag-region>
          {target ? target.name || t("meter.unknownTarget") : untargeted}
        </span>
        <span className="meter-timer num" data-tauri-drag-region>
          {formatDuration(meter?.battleTimeMs ?? 0)}
        </span>
        <div className="meter-controls">
          <button type="button" className="icon-btn" title={t("meter.details")} onClick={() => void commands.openHub("details", null)}>
            <Icon name="details" />
          </button>
          <button
            type="button"
            className="icon-btn"
            title={t("meter.tracker")}
            aria-pressed={settings.tracker.enabled}
            onClick={() => void commands.setTrackerEnabled(!settings.tracker.enabled)}
          >
            <Icon name="buffs" />
          </button>
          <button type="button" className="icon-btn" title={t("meter.reset")} onClick={() => void commands.resetMeter()}>
            <Icon name="reset" />
          </button>
          <button type="button" className="icon-btn" title={t("meter.settings")} onClick={() => void commands.openHub("settings", null)}>
            <Icon name="settings" />
          </button>
          <button type="button" className="icon-btn" title={t("meter.lock")} onClick={() => void save({ ...settings, clickThrough: true })}>
            <Icon name="lock" />
          </button>
          <button type="button" className="icon-btn" title={t("meter.hide")} onClick={() => void getCurrentWindow().hide()}>
            <Icon name="close" />
          </button>
        </div>
        <span className="meter-locked" title={t("meter.locked")}>
          <Icon name="lock" />
        </span>
      </div>
      {hp !== null && (
        <div className="target-hp">
          <div className="target-hp-track">
            <div className="target-hp-fill" style={{ width: `${hp * 100}%` }} />
          </div>
          <span className="target-hp-value num">{formatPercent(hp, settings.language)}</span>
        </div>
      )}
    </header>
  );
}
