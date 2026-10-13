import type { MeterSnapshot } from "../../lib/bindings";
import { formatAmount } from "../../lib/format";
import { useSettings } from "../../lib/settings";

function pingLevel(ms: number): "good" | "fair" | "slow" | "bad" {
  if (ms < 60) return "good";
  if (ms < 100) return "fair";
  if (ms < 200) return "slow";
  return "bad";
}

export function MeterFooter({ meter }: { meter: MeterSnapshot }) {
  const { settings, t } = useSettings();
  const { status, pingMs } = meter;
  const badge =
    status.kind === "demo" ? t("status.demo") : status.kind === "replay" ? t(status.finished ? "status.replayDone" : "status.replay") : null;

  return (
    <footer className="meter-footer" data-tauri-drag-region>
      {pingMs !== null ? (
        <span className="meter-ping num" data-level={pingLevel(pingMs)}>
          {pingMs} ms
        </span>
      ) : (
        badge && <span className="meter-badge">{badge}</span>
      )}
      {meter.recording && <span className="meter-rec">REC</span>}
      <span className="meter-dungeon" data-tauri-drag-region>
        {meter.dungeon ?? ""}
      </span>
      {meter.partyDps > 0 && (
        <span className="meter-party num">
          <span className="meter-party-label">{t("meter.party")}</span>
          {formatAmount(meter.partyDps, settings.meter.numberFormat, settings.language)}/s
        </span>
      )}
    </footer>
  );
}
