import { type CSSProperties, type ReactNode, useEffect, useState } from "react";

/** A labelled setting: name and hint on the left, its control on the right. */
export function SettingRow({ label, hint, children }: { label: string; hint?: string | undefined; children: ReactNode }) {
  return (
    <div className="setting-row">
      <div className="setting-label">
        <span>{label}</span>
        {hint && <span className="setting-hint">{hint}</span>}
      </div>
      <div className="setting-control">{children}</div>
    </div>
  );
}

export function Checkbox({ checked, label, onChange }: { checked: boolean; label: string; onChange: (next: boolean) => void }) {
  return <button type="button" role="checkbox" aria-checked={checked} aria-label={label} className="check" onClick={() => onChange(!checked)} />;
}

/**
 * A slider that commits once the thumb is released, so dragging does not
 * write the settings file on every step.
 */
export function Slider({
  value,
  min,
  max,
  step,
  label,
  format,
  onCommit,
}: {
  value: number;
  min: number;
  max: number;
  step: number;
  label: string;
  format: (value: number) => string;
  onCommit: (value: number) => void;
}) {
  const [draft, setDraft] = useState(value);
  useEffect(() => setDraft(value), [value]);
  const commit = () => {
    if (draft !== value) onCommit(draft);
  };
  const fill = `${((draft - min) / (max - min)) * 100}%`;

  return (
    <div className="slider">
      <input
        type="range"
        className="range"
        aria-label={label}
        min={min}
        max={max}
        step={step}
        value={draft}
        style={{ "--fill": fill } as CSSProperties}
        onChange={(e) => setDraft(Number(e.currentTarget.value))}
        onPointerUp={commit}
        onKeyUp={commit}
        onBlur={commit}
      />
      <span className="slider-value num">{format(draft)}</span>
    </div>
  );
}

export function Choice<T extends string>({
  value,
  options,
  label,
  onChange,
}: {
  value: T;
  options: ReadonlyArray<readonly [T, string]>;
  label: string;
  onChange: (next: T) => void;
}) {
  return (
    <select className="field" aria-label={label} value={value} onChange={(e) => onChange(e.currentTarget.value as T)}>
      {options.map(([id, name]) => (
        <option key={id} value={id}>
          {name}
        </option>
      ))}
    </select>
  );
}
