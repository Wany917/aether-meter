/** "CommandOrControl+Shift+KeyL" -> ["Ctrl", "Shift", "L"]. */
export function keycaps(value: string): string[] {
  return value.split("+").map((part) =>
    part === "CommandOrControl" ? "Ctrl" : part.replace(/^Key(?=[A-Z]$)/, "").replace(/^Digit(?=\d$)/, ""),
  );
}
