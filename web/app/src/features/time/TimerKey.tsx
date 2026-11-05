import { useShortcuts } from "../../shared/use-shortcuts.ts";
import type { EntryIssue } from "./entry-issue.ts";
import { useEntryDefaults } from "./use-entry-defaults.ts";
import { useTimerOn } from "./use-timer-on.ts";

/**
 * The key `t` on an open issue (UX §6.2): starts the timer on it, or opens *Stop timer* while it runs
 * there. It works only where the person may book time, like the buttons that do the same.
 */
export function TimerKey({ issue }: { readonly issue: EntryIssue }) {
  const allowed = useEntryDefaults(issue.key).data !== undefined;
  const { press } = useTimerOn(issue.key);
  useShortcuts(allowed ? { timer: press } : {});
  return null;
}
