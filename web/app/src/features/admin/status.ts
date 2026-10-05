import type { Schema } from "@bugaboo/api-client";

import { daysUntil } from "../../shared/format.ts";

type State = Schema<"ServerStatusBody">;
type SecurityLog = Schema<"SecurityLogBody">;
type DataFolderOpen = Schema<"DataFolderOpenBody">;

const MS_PER_SECOND = 1000;

/** How many days before the end of the certificate the status page starts to warn. */
export const CERTIFICATE_WARNING_DAYS = 30;

/** How serious the state of the server is: nothing to do, something to look at, or broken. */
export type Level = "ok" | "warning" | "problem";

/** Whether nothing vouches for the end of a log that has entries: a warning (M1-F20). */
export function isUnanchored(log: SecurityLog): boolean {
  return log.result === "valid" && (log.entries ?? 0) > 0 && log.anchored === false;
}

/** Whether entries of the log could not be written since the server started: a warning (M1-F40). */
export function hasWriteFailures(log: SecurityLog): boolean {
  return log.write_failures > 0;
}

/** The parts of the data folder that other accounts on this computer can open (SEC-16, M1-F65). */
export function openParts(open: DataFolderOpen): readonly (keyof DataFolderOpen)[] {
  const parts: readonly (keyof DataFolderOpen)[] = [
    "folder",
    "database",
    "files",
    "backups",
    "setup_code_file",
  ];
  return parts.filter((part) => open[part] === "open");
}

/**
 * What time it is for the server: when it started plus how long it has run. The status page
 * judges the certificate by this and not by the clock of the browser, which may be wrong, and
 * which a page that must render the same way twice may not ask.
 */
export function serverNow(state: State): number {
  return Date.parse(state.started_at) + state.uptime_seconds * MS_PER_SECOND;
}

/** The days left of the certificate at `nowMs`; `null` when the server does not show one. */
export function certificateDays(state: State, nowMs: number): number | null {
  const { certificate } = state;
  return certificate === null || certificate === undefined
    ? null
    : daysUntil(certificate.expires_at, nowMs);
}

/** How serious the state is at `nowMs`. */
export function levelOf(state: State, nowMs: number): Level {
  const days = certificateDays(state, nowMs);
  const { database, security_log: log } = state;
  if (!database.ok || log.result !== "valid" || (days !== null && days < 0)) {
    return "problem";
  }
  if (
    isUnanchored(log) ||
    hasWriteFailures(log) ||
    openParts(state.data_folder_open).length > 0 ||
    (days !== null && days < CERTIFICATE_WARNING_DAYS)
  ) {
    return "warning";
  }
  return "ok";
}
