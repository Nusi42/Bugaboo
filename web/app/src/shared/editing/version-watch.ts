import type { RecordRef } from "@bugaboo/api-client";

/** A record on the screen and the version the screen has of it. */
export interface Watched {
  readonly record: RecordRef;
  readonly version: number;
}

/** The version the server has now of a record asked about; `null` when it is gone or not for the person. */
export interface Found {
  readonly record: RecordRef;
  readonly version?: number | null | undefined;
}

/**
 * The records whose version on the server is newer than the one on the screen (ADR-0019): somebody
 * else changed them. A record that is gone or that the person may not know about is not a change
 * the screen can show, so it is left out.
 */
export function newerThan(watched: readonly Watched[], found: readonly Found[]): Watched[] {
  return watched.flatMap((one, at) => {
    const now = found[at]?.version;
    return now !== undefined && now !== null && now > one.version
      ? [{ record: one.record, version: now }]
      : [];
  });
}

/** Coming back to a tab fires several events together; one check answers all of them. */
const ONE_CHECK_WITHIN_MS = 1000;

/** Whether the last check was so near `now` (both in milliseconds) that another one is not needed. */
export function checkedJustNow(lastCheck: number, now: number): boolean {
  return now - lastCheck < ONE_CHECK_WITHIN_MS;
}
