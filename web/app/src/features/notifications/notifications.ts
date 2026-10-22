import type { Schema } from "@bugaboo/api-client";

import type { Wording, WordedNotice } from "@bugaboo/wasm";

import { whoOf, wordedChange } from "../../shared/change-sentence.ts";
import { wordedFile } from "../../shared/file-sentence.ts";
import { renderSaid } from "../../shared/said.ts";
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

/** The short name of the project whose issue IDs could not be renewed, if the person may know it. */
export function failedRenewalProjectOf(row: NotificationRow): string | undefined {
  const renewal = row.entries.find(
    (entry) => entry.issue_ids_failed !== undefined && entry.issue_ids_failed !== null,
  )?.issue_ids_failed;
  return renewal?.shown === "visible" ? (renewal.short_name ?? undefined) : undefined;
}

/** What a notification says, as the shared wording takes it (`bb-core`, `wording::Notice`). */
export function noticeOf(row: NotificationRow): WordedNotice {
  const edit = row.entries.find((entry) => entry.notice !== undefined)?.notice;
  const renewal = row.entries.find(
    (entry) => entry.issue_ids_failed !== undefined && entry.issue_ids_failed !== null,
  )?.issue_ids_failed;
  const file = row.entries.find((entry) => entry.file_event !== undefined);
  return {
    kind: row.kind,
    who: whoOf(row),
    comment: commentOf(row) !== undefined,
    changes: isChange(row) ? row.entries.map((entry) => wordedChange(entry)) : [],
    notice: edit ?? null,
    run_done: !row.entries.some((entry) => entry.run_done === false),
    file: file === undefined ? null : wordedFile(file).what,
    renewal:
      renewal === undefined || renewal === null
        ? null
        : { project: renewal.shown === "visible" ? (renewal.name ?? null) : null },
  };
}

/** What happened, in words: one sentence, or one for each field a change touched. */
export function sentencesOf(wording: Wording, row: NotificationRow): readonly string[] {
  return wording.notificationSentences(noticeOf(row)).map((said) => renderSaid(said));
}
