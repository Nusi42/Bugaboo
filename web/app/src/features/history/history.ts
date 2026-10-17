import { historyTabWords, type Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";

import { api } from "../../shared/api.ts";
import { pageSize, usePaged } from "../../shared/paged.ts";
import { changeValueText } from "../../shared/change-value.ts";
import { fieldNameOf } from "../../shared/field-names.ts";
import { personName } from "../../shared/person.ts";
import { queryKeys } from "../../shared/query-keys.ts";

/** One item of the history of an issue: a comment, one field that changed, or a relation. */
export type HistoryEntry = Schema<"HistoryEntryItem">;

/** The tabs of the history, in the order they are shown (the words the server lists). */
export const historyTabs = historyTabWords;
export type HistoryTabName = Schema<"HistoryTab">;

/** The history of an issue, newest first, a page at a time. */
export function useHistory(issue: string, tab: HistoryTabName) {
  return usePaged({
    queryKey: queryKeys.history.list(issue, tab),
    read: (after) =>
      api.client.GET("/api/v1/issues/{issue}/history", {
        params: { path: { issue }, query: { tab, limit: pageSize.short, ...after } },
      }),
  });
}

/** The items of one event, which the server lists one after the other, as one entry. */
export function groupByEvent(items: readonly HistoryEntry[]): readonly (readonly HistoryEntry[])[] {
  const groups: HistoryEntry[][] = [];
  for (const item of items) {
    const last = groups.at(-1);
    if (last !== undefined && last[0]?.event === item.event && item.kind === "change") {
      last.push(item);
    } else {
      groups.push([item]);
    }
  }
  return groups;
}

/** Who did it, at the start of a sentence; Bugaboo itself when nobody is named. */
export function whoOf(entry: Schema<"HistoryEntryItem">): string {
  return entry.by === undefined || entry.by === null ? t("history.system") : personName(entry.by);
}

/** The value of one side of a history entry in the one shape of a change's value, if it has any. */
function valueOf(entry: HistoryEntry, side: "before" | "after"): Schema<"ChangeValueBody"> | null {
  const before = side === "before";
  const text = before ? entry.before_text : entry.after_text;
  const value = before ? entry.before_value : entry.after_value;
  const names = before ? entry.before_names : entry.after_names;
  const people = before ? entry.before_people : entry.after_people;
  const issue = before ? entry.before_issue : entry.after_issue;
  const project = before ? entry.before_project : entry.after_project;
  if (text !== undefined && text !== null) {
    return { shown: "text", text };
  }
  if (
    value !== undefined &&
    value !== null &&
    entry.field_kind !== undefined &&
    entry.field_kind !== null
  ) {
    return { shown: "typed", kind: entry.field_kind, value };
  }
  if (names !== undefined && names !== null) {
    return { shown: "names", names };
  }
  if (people !== undefined && people !== null) {
    return { shown: "people", people };
  }
  if (project !== undefined && project !== null) {
    return { shown: "project", project };
  }
  return issue === undefined || issue === null ? null : { shown: "issue", issue };
}

/** What one side of a change shows: a word, the people, or the issue; nothing for no value. */
export function sideOf(entry: HistoryEntry, side: "before" | "after"): string | undefined {
  return changeValueText(entry.field ?? "", valueOf(entry, side));
}

/** The sentence of a change of one field: *Ben changed Priority from Normal to High*. */
export function changeSentence(entry: HistoryEntry): string {
  const who = whoOf(entry);
  const field = fieldNameOf({ code: entry.field ?? "", name: entry.field_name });
  const before = sideOf(entry, "before");
  const after = sideOf(entry, "after");
  if (before !== undefined && after !== undefined) {
    return t("history.changed", { who, field, before, after });
  }
  if (after !== undefined) {
    // People are told after a colon: a field such as *Assigned to* ends in the word that *set … to* needs.
    return entry.after_people === undefined || entry.after_people === null
      ? t("history.set", { who, field, after })
      : t("history.set-people", { who, field, after });
  }
  return before === undefined
    ? t("history.changed-long-text", { who, field })
    : t("history.cleared", { who, field });
}

const fileMessages = {
  added: "history.file-added",
  deleted: "history.file-deleted",
  restored: "history.file-restored",
} as const;

/** The sentence of a file that was added, deleted or brought back: *Ben added the file jam.txt*. */
export function fileSentence(entry: HistoryEntry): string {
  const event =
    entry.file_event === "deleted" || entry.file_event === "restored" ? entry.file_event : "added";
  return t(fileMessages[event], { who: whoOf(entry), name: entry.file_name ?? "" });
}

/** The sentence of one item that is not a comment: a change of a field, a file or a relation. */
export function sentenceOf(entry: HistoryEntry): string {
  if (entry.kind === "relation") {
    return relationSentence(entry);
  }
  return entry.kind === "file" ? fileSentence(entry) : changeSentence(entry);
}

/** The sentence of a relation that was added or taken away. */
export function relationSentence(entry: HistoryEntry): string {
  const message =
    entry.relation === "added" ? "history.relation-added" : "history.relation-removed";
  return t(message, {
    who: whoOf(entry),
    name: entry.field ?? "",
    issue: entry.issue?.key ?? "",
  });
}
