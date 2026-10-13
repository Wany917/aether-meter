import { type AppError, commands } from "../../lib/bindings";

type Result<T> = { status: "ok"; data: T } | { status: "error"; error: AppError };

/** Run a command whose new view comes back through an event; report its error. */
export function run<T>(command: Promise<Result<T>>, onError: (message: string) => void): void {
  void command.then((result) => {
    if (result.status === "error") onError(result.error.message);
  });
}

export function openLink(url: string): void {
  void commands.openLink(url);
}
