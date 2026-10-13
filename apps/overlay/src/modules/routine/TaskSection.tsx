import { type CSSProperties, useState } from "react";

import { type Cadence, commands, type TaskGroup, type TaskView } from "../../lib/bindings";
import { formatWeekday } from "../../lib/format";
import { useSettings } from "../../lib/settings";
import { Icon } from "../../ui/Icon";
import { openLink, run } from "./run";

const NEW_TASK_CADENCE: Record<TaskGroup, Cadence> = {
  daily: { kind: "daily" },
  weekly: { kind: "weekly", day: "wednesday" },
  shop: { kind: "weekly", day: "wednesday" },
};

function TaskRow({ task, onError }: { task: TaskView; onError: (m: string) => void }) {
  const { settings, t } = useSettings();
  const done = task.progress >= task.target;
  const single = task.target === 1;
  // Weekly content that does not turn over with the Wednesday reset says when it does.
  const offDay = task.cadence.kind === "weekly" && task.cadence.day !== "wednesday";

  return (
    <li
      className="task plate"
      data-done={done || undefined}
      style={{ "--fill": `${(task.progress / task.target) * 100}%` } as CSSProperties}
    >
      <span className="plate-bar" aria-hidden="true" />
      <button
        type="button"
        role="checkbox"
        aria-checked={done}
        aria-label={task.label}
        className="check"
        // The box completes the task at once, or clears it; the counter steps by one.
        onClick={() => run(commands.routineStep(task.id, done ? -task.progress : task.target - task.progress), onError)}
      />
      <div className="task-text">
        <span className="task-name">{task.label}</span>
        {(task.note || offDay || !task.verified) && (
          <span className="task-note">
            {offDay && `${t("routine.resets")} ${formatWeekday(task.nextResetMs, settings.language)}. `}
            {task.note}
            {!task.verified && <span className="unverified"> {t("routine.unverified")}</span>}
          </span>
        )}
      </div>
      {task.guide && (
        <button type="button" className="text-btn" onClick={() => openLink(task.guide ?? "")}>
          {t("routine.guide")}
        </button>
      )}
      {!single && (
        <span className="task-count">
          <button type="button" className="step-btn" aria-label="-1" disabled={task.progress === 0} onClick={() => run(commands.routineStep(task.id, -1), onError)}>
            <Icon name="minimize" />
          </button>
          <span className="num">
            {task.progress}/{task.target}
          </span>
          <button type="button" className="step-btn" aria-label="+1" disabled={done} onClick={() => run(commands.routineStep(task.id, 1), onError)}>
            <Icon name="plus" />
          </button>
        </span>
      )}
      <button
        type="button"
        className="row-action"
        title={task.custom ? t("routine.delete") : t("routine.hide")}
        onClick={() => run(commands.routineRemove(task.id), onError)}
      >
        <Icon name="close" />
      </button>
    </li>
  );
}

export function TaskSection({
  title,
  group,
  tasks,
  onError,
}: {
  title: string;
  group: TaskGroup;
  tasks: TaskView[];
  onError: (m: string) => void;
}) {
  const { t } = useSettings();
  const [label, setLabel] = useState("");
  const [target, setTarget] = useState("1");
  const doneCount = tasks.filter((task) => task.progress >= task.target).length;

  const add = () => {
    const count = Number.parseInt(target, 10);
    if (!label.trim() || !Number.isFinite(count)) return;
    run(commands.routineAddTask(label.trim(), group, NEW_TASK_CADENCE[group], count), onError);
    setLabel("");
    setTarget("1");
  };

  return (
    <section className="task-section">
      <h2 className="routine-title">
        {title}
        <span className="routine-count num">
          {doneCount}/{tasks.length}
        </span>
      </h2>
      <ol className="task-list">
        {tasks.map((task) => (
          <TaskRow key={task.id} task={task} onError={onError} />
        ))}
      </ol>
      <form
        className="task-add"
        onSubmit={(e) => {
          e.preventDefault();
          add();
        }}
      >
        <input className="field" placeholder={t("routine.newTask")} value={label} maxLength={60} onChange={(e) => setLabel(e.currentTarget.value)} />
        <input
          className="field num task-add-count"
          inputMode="numeric"
          aria-label="×"
          value={target}
          onChange={(e) => setTarget(e.currentTarget.value.replace(/\D/g, "").slice(0, 3))}
        />
        <button type="submit" className="btn btn-slate" disabled={!label.trim()}>
          {t("routine.add")}
        </button>
      </form>
    </section>
  );
}
