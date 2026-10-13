import { useState } from "react";

import { commands, type Settings, type TargetMode } from "../../lib/bindings";
import { formatPercent } from "../../lib/format";
import { LANGUAGE_NAMES, targetModeName } from "../../lib/i18n";
import { CommandError } from "../../lib/ipc";
import { useSettings } from "../../lib/settings";
import { Checkbox, Choice, SettingRow, Slider } from "./controls";
import { HiddenTimers } from "./HiddenTimers";
import { HotkeyField } from "./HotkeyField";

const TARGET_MODES: TargetMode[] = ["boss", "mostDamage", "mostRecent", "lastHitByMe", "allTargets"];

export function SettingsTab() {
  const { settings, t, save } = useSettings();
  const [error, setError] = useState<string | null>(null);
  const { meter, tracker, hotkeys, language } = settings;

  const apply = (next: Settings) => {
    setError(null);
    save(next).catch((e: unknown) => setError(e instanceof CommandError ? e.message : String(e)));
  };
  const setMeter = (change: Partial<Settings["meter"]>) => apply({ ...settings, meter: { ...meter, ...change } });
  const setTracker = (change: Partial<Settings["tracker"]>) => apply({ ...settings, tracker: { ...tracker, ...change } });
  const setHotkeys = (change: Partial<Settings["hotkeys"]>) => apply({ ...settings, hotkeys: { ...hotkeys, ...change } });
  const percent = (v: number) => formatPercent(v, language, 0);

  return (
    <div className="settings">
      {error && (
        <p className="settings-error" role="alert">
          <strong>{t("settings.error")}</strong> {error}
        </p>
      )}

      <section className="settings-section">
        <h2>{t("settings.general")}</h2>
        <SettingRow label={t("settings.language")} hint={t("settings.languageHint")}>
          <Choice value={language} options={LANGUAGE_NAMES} label={t("settings.language")} onChange={(v) => apply({ ...settings, language: v })} />
        </SettingRow>
        <SettingRow label={t("settings.targetMode")} hint={t("settings.targetModeHint")}>
          <Choice
            value={settings.targetMode}
            options={TARGET_MODES.map((m) => [m, targetModeName(m, language)] as const)}
            label={t("settings.targetMode")}
            onChange={(v) => apply({ ...settings, targetMode: v })}
          />
        </SettingRow>
        <SettingRow label={t("settings.clickThrough")} hint={t("settings.clickThroughHint")}>
          <Checkbox checked={settings.clickThrough} label={t("settings.clickThrough")} onChange={(v) => apply({ ...settings, clickThrough: v })} />
        </SettingRow>
      </section>

      <section className="settings-section">
        <h2>{t("settings.meter")}</h2>
        <SettingRow label={t("settings.opacity")}>
          <Slider value={meter.opacity} min={0.2} max={1} step={0.02} label={t("settings.opacity")} format={percent} onCommit={(v) => setMeter({ opacity: v })} />
        </SettingRow>
        <SettingRow label={t("settings.scale")}>
          <Slider value={meter.scale} min={0.8} max={1.5} step={0.05} label={t("settings.scale")} format={percent} onCommit={(v) => setMeter({ scale: v })} />
        </SettingRow>
        <SettingRow label={t("settings.maxRows")}>
          <Slider value={meter.maxRows} min={3} max={12} step={1} label={t("settings.maxRows")} format={String} onCommit={(v) => setMeter({ maxRows: v })} />
        </SettingRow>
        <SettingRow label={t("settings.numberFormat")}>
          <Choice
            value={meter.numberFormat}
            options={[
              ["compact", t("settings.numberFormat.compact")],
              ["full", t("settings.numberFormat.full")],
            ]}
            label={t("settings.numberFormat")}
            onChange={(v) => setMeter({ numberFormat: v })}
          />
        </SettingRow>
        <SettingRow label={t("settings.nameDisplay")} hint={t("settings.nameDisplayHint")}>
          <Choice
            value={meter.nameDisplay}
            options={[
              ["full", t("settings.nameDisplay.full")],
              ["masked", t("settings.nameDisplay.masked")],
              ["class", t("settings.nameDisplay.class")],
            ]}
            label={t("settings.nameDisplay")}
            onChange={(v) => setMeter({ nameDisplay: v })}
          />
        </SettingRow>
        <SettingRow label={t("settings.showCombatPower")} hint={t("settings.showCombatPowerHint")}>
          <Checkbox checked={meter.showCombatPower} label={t("settings.showCombatPower")} onChange={(v) => setMeter({ showCombatPower: v })} />
        </SettingRow>
      </section>

      <section className="settings-section">
        <h2>{t("settings.tracker")}</h2>
        <SettingRow label={t("settings.trackerEnabled")}>
          <Checkbox checked={tracker.enabled} label={t("settings.trackerEnabled")} onChange={(v) => setTracker({ enabled: v })} />
        </SettingRow>
        <SettingRow label={t("settings.showBuffs")}>
          <Checkbox checked={tracker.showBuffs} label={t("settings.showBuffs")} onChange={(v) => setTracker({ showBuffs: v })} />
        </SettingRow>
        <SettingRow label={t("settings.showCooldowns")}>
          <Checkbox checked={tracker.showCooldowns} label={t("settings.showCooldowns")} onChange={(v) => setTracker({ showCooldowns: v })} />
        </SettingRow>
        <SettingRow label={t("settings.iconSize")}>
          <Slider value={tracker.iconSize} min={24} max={64} step={2} label={t("settings.iconSize")} format={(v) => `${v} px`} onCommit={(v) => setTracker({ iconSize: v })} />
        </SettingRow>
        <SettingRow label={t("settings.hidden")} hint={tracker.hidden.length === 0 ? t("settings.hiddenEmpty") : undefined}>
          {tracker.hidden.length > 0 && (
            <HiddenTimers keys={tracker.hidden} onRestore={(key) => setTracker({ hidden: tracker.hidden.filter((k) => k !== key) })} />
          )}
        </SettingRow>
      </section>

      <section className="settings-section">
        <h2>{t("settings.hotkeys")}</h2>
        <SettingRow label={t("settings.hotkey.clickThrough")}>
          <HotkeyField value={hotkeys.clickThrough} label={t("settings.hotkey.clickThrough")} onChange={(v) => setHotkeys({ clickThrough: v })} />
        </SettingRow>
        <SettingRow label={t("settings.hotkey.reset")}>
          <HotkeyField value={hotkeys.reset} label={t("settings.hotkey.reset")} onChange={(v) => setHotkeys({ reset: v })} />
        </SettingRow>
        <SettingRow label={t("settings.hotkey.toggleOverlays")}>
          <HotkeyField value={hotkeys.toggleOverlays} label={t("settings.hotkey.toggleOverlays")} onChange={(v) => setHotkeys({ toggleOverlays: v })} />
        </SettingRow>
      </section>

      <section className="settings-section">
        <h2>{t("settings.about")}</h2>
        <p className="about-text">{t("settings.aboutText")}</p>
        <button type="button" className="btn btn-slate" onClick={() => void commands.quit()}>
          {t("settings.quit")}
        </button>
      </section>
    </div>
  );
}
