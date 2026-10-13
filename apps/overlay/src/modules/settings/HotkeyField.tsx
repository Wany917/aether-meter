import { useEffect, useState } from "react";

import { keycaps } from "../../lib/hotkeys";
import { useSettings } from "../../lib/settings";

const MODIFIER_CODES = new Set(["ControlLeft", "ControlRight", "ShiftLeft", "ShiftRight", "AltLeft", "AltRight", "MetaLeft", "MetaRight"]);

/** An accelerator the global-shortcut plugin parses, from a key event. */
function accelerator(e: KeyboardEvent): string | null {
  if (MODIFIER_CODES.has(e.code)) return null;
  const modifiers = [e.ctrlKey || e.metaKey ? "CommandOrControl" : null, e.altKey ? "Alt" : null, e.shiftKey ? "Shift" : null].filter(
    (m): m is string => m !== null,
  );
  // A bare key would fire while typing in game chat; F-keys are the exception.
  if (modifiers.length === 0 && !/^F\d{1,2}$/.test(e.code)) return null;
  return [...modifiers, e.code].join("+");
}

export function HotkeyField({ value, label, onChange }: { value: string; label: string; onChange: (next: string) => void }) {
  const { t } = useSettings();
  const [recording, setRecording] = useState(false);

  useEffect(() => {
    if (!recording) return;
    const onKey = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();
      if (e.code === "Escape") return setRecording(false);
      if (e.code === "Backspace" || e.code === "Delete") {
        setRecording(false);
        return onChange("");
      }
      const next = accelerator(e);
      if (next) {
        setRecording(false);
        onChange(next);
      }
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [recording, onChange]);

  return (
    <button
      type="button"
      className="hotkey-field"
      data-recording={recording || undefined}
      aria-label={label}
      onClick={() => setRecording((r) => !r)}
      onBlur={() => setRecording(false)}
    >
      {recording ? (
        <span className="hotkey-prompt">{t("settings.hotkey.record")}</span>
      ) : value ? (
        keycaps(value).map((cap, i) => (
          <kbd key={`${cap}${i}`} className="keycap">
            {cap}
          </kbd>
        ))
      ) : (
        <span className="hotkey-prompt">{t("settings.hotkey.none")}</span>
      )}
    </button>
  );
}
