import { refusedOf, type Refused } from "../../shared/refused.ts";
import { inOrder } from "../../shared/in-order.ts";
import { sendChange } from "./issue-requests.ts";
import type { IssueRequestValues } from "./issue-values.ts";

/**
 * The saves of dates of the screens that change them at once (the timeline, the calendar): the changes of
 * one issue go one after the other, each on the version the one before made, and a screen that never read
 * the version a change made (a clash) takes it with `takeVersion`. One place, so both screens keep the
 * versions the same way (ODR).
 */

/** The newest version of each issue that a save of a screen made. */
const savedVersions = new Map<string, number>();

/** The version a change is made on: what the page read, or what the last save made, whichever is newer. */
export function newestVersion(issue: string, read: number | null | undefined): number {
  return Math.max(read ?? 0, savedVersions.get(issue) ?? 0);
}

/** The version a change of somebody else made: *Use mine* of a clash saves the change of the person on it. */
export function takeVersion(issue: string, read: number | null | undefined, version: number): void {
  savedVersions.set(issue, Math.max(newestVersion(issue, read), version));
}

/** The queue of the saves of the dates of one issue. */
function queueOf(issue: string): string {
  return `dates:${issue}`;
}

/**
 * Saves values of an issue after every save of it that a screen asked for before, on the version the one
 * before made. A refusal comes back as the reason of the server in words, or as the change somebody made
 * first, with its version.
 */
export function saveOnNewest(
  issue: string,
  read: number | null | undefined,
  values: IssueRequestValues,
  keepalive = false,
): Promise<Refused | null> {
  return inOrder(queueOf(issue), async () => {
    try {
      const version = await sendChange(issue, {
        version: newestVersion(issue, read),
        values,
        keepalive,
      });
      if (version !== undefined) {
        savedVersions.set(issue, version);
      }
      return null;
    } catch (error) {
      return refusedOf(error);
    }
  });
}

/** Runs `work` after every save of the issue that a screen asked for, with the version the last of them made. */
export function afterSaves<T>(
  issue: string,
  read: number | null | undefined,
  work: (version: number) => Promise<T>,
): Promise<T> {
  return inOrder(queueOf(issue), () => work(newestVersion(issue, read)));
}

/** Runs a change of an issue after every save, and keeps the version it made. */
export async function changeAfterSaves(
  issue: string,
  read: number | null | undefined,
  change: (version: number) => Promise<number | undefined>,
): Promise<void> {
  const made = await afterSaves(issue, read, change);
  if (made !== undefined) {
    savedVersions.set(issue, made);
  }
}
