import type { NumberFormat } from "./bindings";
import { uiLocale } from "./i18n";

const formatters = new Map<string, Intl.NumberFormat>();

function numberFormat(language: string, options: Intl.NumberFormatOptions): Intl.NumberFormat {
  const key = `${uiLocale(language)}|${JSON.stringify(options)}`;
  let formatter = formatters.get(key);
  if (!formatter) {
    formatter = new Intl.NumberFormat(uiLocale(language), options);
    formatters.set(key, formatter);
  }
  return formatter;
}

/** Damage, DPS, HP: "1,2 M" or "1 234 567" depending on the setting. */
export function formatAmount(value: number, format: NumberFormat, language: string): string {
  if (format === "full" || Math.abs(value) < 1_000) {
    return numberFormat(language, { maximumFractionDigits: 0 }).format(Math.round(value));
  }
  return numberFormat(language, { notation: "compact", maximumFractionDigits: 1 }).format(value);
}

export function formatPercent(ratio: number, language: string, digits = 1): string {
  return numberFormat(language, {
    style: "percent",
    minimumFractionDigits: digits,
    maximumFractionDigits: digits,
  }).format(Number.isFinite(ratio) ? ratio : 0);
}

/** 2:05, or 1:02:05 past an hour. */
export function formatDuration(ms: number): string {
  const total = Math.max(0, Math.floor(ms / 1000));
  const hours = Math.floor(total / 3600);
  const minutes = Math.floor((total % 3600) / 60);
  const seconds = String(total % 60).padStart(2, "0");
  return hours > 0 ? `${hours}:${String(minutes).padStart(2, "0")}:${seconds}` : `${minutes}:${seconds}`;
}

/** Countdown under an icon: "12", then "4.2" for the last seconds. */
export function formatCountdown(ms: number): string {
  const seconds = Math.max(0, ms) / 1000;
  if (seconds >= 60) return `${Math.ceil(seconds / 60)}m`;
  if (seconds >= 5) return String(Math.ceil(seconds));
  return seconds.toFixed(1);
}

export function formatTime(epochMs: number, language: string): string {
  return new Intl.DateTimeFormat(uiLocale(language), { hour: "2-digit", minute: "2-digit" }).format(epochMs);
}

export function formatDay(epochMs: number, language: string): string {
  return new Intl.DateTimeFormat(uiLocale(language), { weekday: "long", day: "numeric", month: "long" }).format(epochMs);
}

/** "Jinguu" -> "J*****". */
export function maskName(name: string): string {
  const [first = "", ...rest] = Array.from(name);
  return first + "*".repeat(Math.min(rest.length, 8));
}

const englishOrdinals = new Intl.PluralRules("en", { type: "ordinal" });
const englishSuffixes: Record<string, string> = { one: "st", two: "nd", few: "rd", other: "th" };

/** 1er, 2e / 1st, 2nd. */
export function formatOrdinal(n: number, language: string): string {
  if (uiLocale(language) === "fr") return n === 1 ? "1er" : `${n}e`;
  return `${n}${englishSuffixes[englishOrdinals.select(n)] ?? "th"}`;
}

/** "2 j 5 h", "5 h 12", "12 min" (FR) / "2d 5h", "5h 12m", "12m" (EN). */
export function formatRemaining(ms: number, language: string): string {
  const fr = uiLocale(language) === "fr";
  const minutes = Math.max(0, Math.ceil(ms / 60_000));
  const days = Math.floor(minutes / 1440);
  const hours = Math.floor((minutes % 1440) / 60);
  const mins = minutes % 60;
  if (days > 0) return fr ? `${days} j ${hours} h` : `${days}d ${hours}h`;
  if (hours > 0) return fr ? `${hours} h ${String(mins).padStart(2, "0")}` : `${hours}h ${mins}m`;
  return fr ? `${mins} min` : `${mins}m`;
}

export function formatWeekday(epochMs: number, language: string): string {
  return new Intl.DateTimeFormat(uiLocale(language), { weekday: "long" }).format(epochMs);
}

/** "ven. 21:00": a moment in the week, short. */
export function formatSlot(epochMs: number, language: string): string {
  return new Intl.DateTimeFormat(uiLocale(language), { weekday: "short", hour: "2-digit", minute: "2-digit" }).format(epochMs);
}

/** "jeudi 21:00": a moment more than a day away. */
export function formatWhen(epochMs: number, language: string): string {
  return new Intl.DateTimeFormat(uiLocale(language), { weekday: "long", hour: "2-digit", minute: "2-digit" }).format(epochMs);
}

const DAY_MS = 86_400_000;

/** "dans 5 h 12" within a day, "jeudi 21:00" beyond: a countdown in days reads poorly. */
export function formatUntil(epochMs: number, now: number, language: string, within: string): string {
  return epochMs - now <= DAY_MS ? `${within} ${formatRemaining(epochMs - now, language)}` : formatWhen(epochMs, language);
}
