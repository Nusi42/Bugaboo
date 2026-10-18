import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";

import { changeSentence, whoOf } from "../../shared/change-sentence.ts";
import { fileSentence } from "../../shared/file-sentence.ts";
import type { IssueTarget } from "../../shared/paths.ts";

/** One item of the list of notifications; a change has one for each field it changed. */
export type NotificationItem = Schema<"NotificationEntry">;

/** One notification: the items that share its ID. */
export interface NotificationRow {
  readonly id: string;
  readonly kind: NotificationItem["kind"];
  readonly at: string;
  readonly read: boolean;
  readonly issue: NotificationItem["issue"];
  readonly by: NotificationItem["by"];
  readonly entries: readonly NotificationItem[];
}

/** One row from the items that share its ID. */
function rowOf(entries: readonly NotificationItem[]): NotificationRow | undefined {
  const first = entries[0];
  return first === undefined
    ? undefined
    : {
        id: first.id,
        kind: first.kind,
        at: first.at,
        read: first.read,
        issue: first.issue,
        by: first.by,
        entries,
      };
}

/** The notifications of a list of items, in the order of the list (newest first). */
export function rowsOf(items: readonly NotificationItem[]): readonly NotificationRow[] {
  const byId = new Map<string, readonly NotificationItem[]>();
  for (const item of items) {
    byId.set(item.id, [...(byId.get(item.id) ?? []), item]);
  }
  const rows: NotificationRow[] = [];
  for (const entries of byId.values()) {
    const row = rowOf(entries);
    if (row !== undefined) {
      rows.push(row);
    }
  }
  return rows;
}

/** The notifications about one issue (or about none), newest first. */
export interface IssueGroup {
  /** The readable ID of the issue; the ID of the first notification for one that has no issue. */
  readonly key: string;
  readonly issue: NotificationItem["issue"];
  readonly rows: readonly NotificationRow[];
}

/** Groups the rows by issue; the issue with the newest notification comes first. */
export function groupsOf(rows: readonly NotificationRow[]): readonly IssueGroup[] {
  const byKey = new Map<string, readonly NotificationRow[]>();
  for (const row of rows) {
    const key = row.issue?.key ?? row.id;
    byKey.set(key, [...(byKey.get(key) ?? []), row]);
  }
  const groups: IssueGroup[] = [];
  for (const [key, inGroup] of byKey) {
    groups.push({ key, issue: inGroup[0]?.issue ?? null, rows: inGroup });
  }
  return groups;
}

/** The ID of the comment a row is about, if it is about one. */
function commentOf(row: NotificationRow): string | undefined {
  return (
    row.entries.find((entry) => entry.comment !== undefined && entry.comment !== null)?.comment ??
    undefined
  );
}

/** Whether a row says that fields changed. */
function isChange(row: NotificationRow): boolean {
  return row.entries.some((entry) => entry.field !== undefined && entry.field !== null);
}

/** Where *Open* goes on the issue page: the exact comment or change, if the row has one. */
export function targetOf(row: NotificationRow): IssueTarget | undefined {
  const comment = commentOf(row);
  if (comment !== undefined) {
    return { kind: "comment", comment };
  }
  return isChange(row) ? { kind: "change", at: row.at } : undefined;
}

/** What happened, in words: one sentence, or one for each field a change touched. */
export function sentencesOf(row: NotificationRow): readonly string[] {
  const who = whoOf(row);
  if (row.kind === "mentioned") {
    return [t("notifications.mentioned", { who })];
  }
  if (row.kind === "assigned") {
    return [t("notifications.assigned", { who })];
  }
  if (row.kind === "edit_notice") {
    return [editNoticeSentence(row, who)];
  }
  if (row.kind === "run_ended") {
    const renewal = row.entries.find(
      (entry) => entry.issue_ids_failed !== undefined,
    )?.issue_ids_failed;
    if (renewal !== undefined && renewal !== null) {
      return [
        renewal.shown === "visible" && renewal.name !== undefined && renewal.name !== null
          ? t("notifications.issue-ids-failed", { project: renewal.name })
          : t("notifications.issue-ids-failed-hidden"),
      ];
    }
    return [
      row.entries.some((entry) => entry.run_done === false)
        ? t("notifications.run-failed")
        : t("notifications.run-done"),
    ];
  }
  const file = row.entries.find((entry) => entry.file_event !== undefined);
  if (file !== undefined) {
    return [fileSentence(file)];
  }
  if (isChange(row)) {
    return row.entries.map((entry) => changeSentence(entry));
  }
  return [
    commentOf(row) === undefined ? t("notifications.news") : t("notifications.commented", { who }),
  ];
}

function editNoticeSentence(row: NotificationRow, who: string): string {
  const notice = row.entries.find((entry) => entry.notice !== undefined)?.notice;
  if (notice === "taken_over") {
    return t("notifications.taken-over", { who });
  }
  return notice === "released"
    ? t("notifications.released", { who })
    : t("notifications.finish-asked", { who });
}
