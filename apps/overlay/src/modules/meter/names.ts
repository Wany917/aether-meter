import type { ClassId, NameDisplay } from "../../lib/bindings";
import { maskName } from "../../lib/format";
import { className } from "../../lib/i18n";

/** A player's name as the settings want it shown (streaming, screenshots). */
export function displayName(name: string, classId: ClassId, mode: NameDisplay, language: string): string {
  switch (mode) {
    case "full":
      return name;
    case "masked":
      return maskName(name);
    case "class":
      return className(classId, language);
  }
}
