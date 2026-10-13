import type { EngineStatus } from "../../lib/bindings";
import type { MessageKey } from "../../lib/i18n";
import { useSettings } from "../../lib/settings";

const MESSAGES: Record<EngineStatus["kind"], readonly [MessageKey, MessageKey]> = {
  captureUnavailable: ["status.noCapture", "status.noCaptureHint"],
  waitingForGame: ["status.waiting", "status.waitingHint"],
  live: ["status.ready", "status.readyHint"],
  replay: ["status.replay", "status.readyHint"],
  demo: ["status.demo", "status.readyHint"],
};

/** What the meter shows before anyone has hit anything. */
export function StatusPanel({ status }: { status: EngineStatus }) {
  const { t } = useSettings();
  const [title, hint] = MESSAGES[status.kind];
  return (
    <div className="meter-status" data-kind={status.kind}>
      <span className="status-sigil" aria-hidden="true" />
      <p className="status-title">{t(title)}</p>
      <p className="status-hint">{t(hint)}</p>
      {status.kind === "captureUnavailable" && <p className="status-detail">{status.reason}</p>}
    </div>
  );
}
