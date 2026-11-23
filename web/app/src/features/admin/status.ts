import type { Schema } from "@bugaboo/api-client";

import { daysUntil } from "../../shared/format.ts";

type State = Schema<"ServerStatusBody">;
type SecurityLog = Schema<"SecurityLogBody">;
type DataFolderOpen = Schema<"DataFolderOpenBody">;
type HostOpen = Schema<"HostOpenBody">;
/** The parts of the host whose rights are read; the child of previews is judged on its own. */
export type HostPart = Exclude<keyof HostOpen, "preview_child">;
type PartAccess = DataFolderOpen["folder"];

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

const dataFolderParts: readonly (keyof DataFolderOpen)[] = [
  "folder",
  "database",
  "files",
  "backups",
  "setup_code_file",
];

const hostParts: readonly HostPart[] = ["program", "settings_file", "tls_key"];

/** The parts whose rights are `wanted`, in the order they are shown. */
function partsWith<K extends string>(
  access: Readonly<Record<K, PartAccess>>,
  parts: readonly K[],
  wanted: PartAccess,
): readonly K[] {
  return parts.filter((part) => access[part] === wanted);
}

/** The parts of the data folder that other accounts on this computer can open (SEC-16, M1-F65). */
export function openParts(open: DataFolderOpen): readonly (keyof DataFolderOpen)[] {
  return partsWith(open, dataFolderParts, "open");
}

/** The parts whose rights the server could not read: never taken for private. */
export function unknownParts(open: DataFolderOpen): readonly (keyof DataFolderOpen)[] {
  return partsWith(open, dataFolderParts, "unknown");
}

/** The program, its settings and its HTTPS key, when other accounts can change or read them. */
export function openHostParts(open: HostOpen): readonly HostPart[] {
  return partsWith(open, hostParts, "open");
}

/** The same parts whose rights the server could not read. */
export function unknownHostParts(open: HostOpen): readonly HostPart[] {
  return partsWith(open, hostParts, "unknown");
}

/** Whether the kernel shut the child that makes previews in less than fully (ADR-0032 §4). */
export function previewChildOpen(open: HostOpen): boolean {
  return open.preview_child === "partly" || open.preview_child === "not";
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

/** Whether removals of people stopped before they were finished: a warning (ADR-0044). */
export function hasStoppedRemovals(state: State): boolean {
  return state.removals_stopped > 0;
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
    unknownParts(state.data_folder_open).length > 0 ||
    openHostParts(state.host_open).length > 0 ||
    unknownHostParts(state.host_open).length > 0 ||
    previewChildOpen(state.host_open) ||
    database.certificate_unchecked ||
    hasStoppedRemovals(state) ||
    (days !== null && days < CERTIFICATE_WARNING_DAYS)
  ) {
    return "warning";
  }
  return "ok";
}
