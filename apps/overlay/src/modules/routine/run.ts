import { type AppError, commands, type RoutineView } from "../../lib/bindings";

type Result = { status: "ok"; data: RoutineView } | { status: "error"; error: AppError };

/** Run a routine command; the view comes back through `routine-changed`. */
export function run(command: Promise<Result>, onError: (message: string) => void): void {
  void command.then((result) => {
    if (result.status === "error") onError(result.error.message);
  });
}

export function openLink(url: string): void {
  void commands.openLink(url);
}
