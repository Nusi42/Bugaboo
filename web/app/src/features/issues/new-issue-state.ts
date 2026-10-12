import { openOverlay } from "../shell/index.ts";

/** The project the person looked at last: the new-issue form starts with it. */
let lastShortName: string | undefined;

/** Remembers the project whose issues are on screen. */
export function rememberProject(shortName: string): void {
  lastShortName = shortName;
}

/** The short name of the project the person looked at last, if any. */
export function lastProject(): string | undefined {
  return lastShortName;
}

/** Opens the new-issue dialog, starting with this project when one is named. */
export function openNewIssue(shortName?: string): void {
  if (shortName !== undefined) {
    lastShortName = shortName;
  }
  openOverlay("new-issue");
}
