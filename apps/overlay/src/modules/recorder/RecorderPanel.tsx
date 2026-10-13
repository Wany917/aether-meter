import { type ChangeEvent, useEffect, useState } from "react";

import { commands, type RecorderStatus } from "../../lib/bindings";
import { formatDuration } from "../../lib/format";
import type { MessageKey } from "../../lib/i18n";
import { useSettings } from "../../lib/settings";
import { Choice, SettingRow } from "../settings/controls";

const SCENARIOS = ["cube", "odyle", "duty", "other"] as const;
type Scenario = (typeof SCENARIOS)[number];

const key = (scenario: Scenario, part?: "steps" | "before" | "after") =>
  (part ? `recorder.${scenario}.${part}` : `recorder.${scenario}`) as MessageKey;

const IDLE: RecorderStatus = { active: false, startedMs: null, frames: 0, bytes: 0, full: false };

/** A recording of the game's stream around one action, saved with the values the player saw. */
export function RecorderPanel() {
  const { settings, t } = useSettings();
  const [status, setStatus] = useState<RecorderStatus>(IDLE);
  const [scenario, setScenario] = useState<Scenario>("cube");
  const [before, setBefore] = useState("");
  const [after, setAfter] = useState("");
  const [note, setNote] = useState("");
  const [saved, setSaved] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    void commands.recordingStatus().then(setStatus);
  }, []);
  // The frame count moves while recording.
  useEffect(() => {
    if (!status.active) return;
    const timer = window.setInterval(() => void commands.recordingStatus().then(setStatus), 1000);
    return () => window.clearInterval(timer);
  }, [status.active]);

  const start = () => {
    setSaved(null);
    setError(null);
    void commands.startRecording().then(setStatus);
  };
  const stop = () => {
    const values = [
      [t(key(scenario, "before")), before],
      [t(key(scenario, "after")), after],
    ]
      .filter(([, v]) => v !== "")
      .map(([label, v]) => ({ label: label ?? "", value: Number(v) }));
    void commands.stopRecording({ scenario, values, note: note.trim() }).then((result) => {
      if (result.status === "ok") {
        setSaved(result.data);
        setBefore("");
        setAfter("");
        setNote("");
      } else {
        setError(result.error.message);
      }
      void commands.recordingStatus().then(setStatus);
    });
  };

  const digits = (set: (v: string) => void) => (e: ChangeEvent<HTMLInputElement>) => set(e.currentTarget.value.replace(/\D/g, "").slice(0, 9));
  const options = SCENARIOS.map((s) => [s, t(key(s))] as const);

  return (
    <section className="settings-section recorder">
      <h2>{t("recorder.title")}</h2>
      <p className="setting-hint recorder-hint">{t("recorder.hint")}</p>
      <SettingRow label={t("recorder.scenario")} hint={t(key(scenario, "steps"))}>
        <Choice value={scenario} options={options} label={t("recorder.scenario")} onChange={setScenario} />
      </SettingRow>
      <div className="recorder-values">
        <label>
          <span>{t(key(scenario, "before"))}</span>
          <input className="field num" inputMode="numeric" value={before} onChange={digits(setBefore)} />
        </label>
        <label>
          <span>{t(key(scenario, "after"))}</span>
          <input className="field num" inputMode="numeric" value={after} onChange={digits(setAfter)} />
        </label>
        <label className="recorder-note">
          <span>{t("recorder.note")}</span>
          <input className="field" maxLength={500} value={note} onChange={(e) => setNote(e.currentTarget.value)} />
        </label>
      </div>
      <div className="recorder-actions">
        {status.active ? (
          <>
            <span className="recorder-live num">
              REC {formatDuration(Date.now() - (status.startedMs ?? Date.now()))} · {status.frames.toLocaleString(settings.language)} {t("recorder.frames")}
              {status.full && ` · ${t("recorder.full")}`}
            </span>
            <button type="button" className="btn btn-gold" onClick={stop}>
              {t("recorder.stop")}
            </button>
            <button type="button" className="btn btn-slate" onClick={() => void commands.cancelRecording().then(setStatus)}>
              {t("recorder.cancel")}
            </button>
          </>
        ) : (
          <button type="button" className="btn btn-slate" onClick={start}>
            {t("recorder.start")}
          </button>
        )}
        <button type="button" className="text-btn" onClick={() => void commands.openRecordings()}>
          {t("recorder.folder")}
        </button>
      </div>
      {saved && (
        <p className="setting-hint recorder-saved">
          {t("recorder.saved")} <span className="num">{saved}</span>
        </p>
      )}
      {error && (
        <p className="settings-error" role="alert">
          {error}
        </p>
      )}
    </section>
  );
}
