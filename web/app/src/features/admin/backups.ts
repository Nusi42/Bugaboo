import type { Schema } from "@bugaboo/api-client";
import { t, type MessageKey } from "@bugaboo/messages";

import { api } from "../../shared/api.ts";

type File = Schema<"BackupFileBody">;
type Run = Schema<"BackupRunBody">;
export type Backups = Schema<"BackupsBody">;

/** Who or what made a backup, in the words the server records, plus the two cases it does not record. */
export type Maker =
  | "schedule"
  | "person"
  | "command-line"
  | "upgrade"
  | "before-restore"
  | "unknown";

/** The sentence part for each maker (the words are in the catalog). */
export const makerMessages: Readonly<Record<Maker, MessageKey>> = {
  schedule: "backup-page.maker.daily",
  person: "backup-page.maker.person",
  "command-line": "backup-page.maker.command-line",
  upgrade: "backup-page.maker.upgrade",
  "before-restore": "backup-page.maker.before-restore",
  unknown: "backup-page.maker.unknown",
};

/** The makers the server records for a backup it made (`BackupMaker::code`). */
const recorded = ["schedule", "person", "command-line", "upgrade"] as const;

/** The event of a record that a backup was made, as opposed to restored. */
const MADE = "made";

/**
 * Who made a backup: a backup a restore made of what was there before says so; otherwise the record
 * of the backup with the same name. A file without a record (copied in by hand) has an unknown maker.
 */
export function makerOf(file: File, runs: readonly Run[]): Maker {
  if (file.before_restore) {
    return "before-restore";
  }
  const record = runs.find((run) => run.event === MADE && run.name === file.name);
  return recorded.find((maker) => maker === record?.maker) ?? "unknown";
}

/** One row of the list of backups. */
export interface BackupRow {
  readonly name: string;
  readonly moment: string;
  readonly size: number;
  readonly maker: Maker;
}

/** The rows of the list, in the order of the server: newest first. */
export function backupRows(backups: Backups): readonly BackupRow[] {
  return backups.files.map((file) => ({
    name: file.name,
    moment: file.moment,
    size: file.size,
    maker: makerOf(file, backups.runs),
  }));
}

/** The backup the command to restore is written for: the one chosen while it is in the list, else the newest. */
export function chosenName(rows: readonly BackupRow[], chosen: string | null): string | undefined {
  return rows.find((row) => row.name === chosen)?.name ?? rows[0]?.name;
}

/** The command that restores a backup (one message, so the words are written once). */
export function restoreCommand(name: string): string {
  return t("backup-page.restore-command", { name });
}

/** The query of the list of backups. */
export function backupsOptions() {
  return api.query.queryOptions("get", "/api/v1/admin/backups");
}

/** The step-by-step page of the user docs for a restore. */
export const RESTORE_DOCS =
  "https://github.com/Nusi42/Bugaboo/blob/main/userDocs/install/backups.md#restore-a-backup";
