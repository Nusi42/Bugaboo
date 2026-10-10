import type { Schema } from "@bugaboo/api-client";
import { isMessageKey, t } from "@bugaboo/messages";

import { api } from "../../shared/api.ts";
import { pageSize, usePaged } from "../../shared/paged.ts";
import { personName, personNameInside } from "../../shared/person.ts";
import { queryKeys } from "../../shared/query-keys.ts";

/** One item of the history of an issue: a comment, one field that changed, or a relation. */
export type HistoryEntry = Schema<"HistoryEntryItem">;

/** The tabs of the history, in the order they are shown (the words the server knows). */
export const historyTabs = ["everything", "comments", "changes"] as const;
export type HistoryTabName = (typeof historyTabs)[number];

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

/** The name of a field as people read it: the catalogue's own, or one of the members History adds. */
export function fieldLabel(field: string): string {
  switch (field) {
    case "issue-type":
      return t("history.field-issue-type");
    case "parent":
      return t("history.field-parent");
    case "sub-issue":
      return t("history.field-sub-issue");
    default: {
      const key = `issue-field.${field}`;
      return isMessageKey(key) ? t(key) : t("history.field-other", { field });
    }
  }
}

/** A word of a status or a priority as people read it; any other text as it is. */
function wordOf(field: string, text: string): string {
  const key = field === "status" ? `status.${text}` : `priority.${text}`;
  return (field === "status" || field === "priority") && isMessageKey(key) ? t(key) : text;
}

/** What one side of a change shows: a word, the people, or the issue; nothing for no value. */
export function sideOf(entry: HistoryEntry, side: "before" | "after"): string | undefined {
  const text = side === "before" ? entry.before_text : entry.after_text;
  const people = side === "before" ? entry.before_people : entry.after_people;
  const issue = side === "before" ? entry.before_issue : entry.after_issue;
  if (text !== undefined && text !== null) {
    return wordOf(entry.field ?? "", text);
  }
  if (people !== undefined && people !== null) {
    return people.length === 0
      ? t("history.people-none")
      : people.map((person) => personNameInside(person)).join(", ");
  }
  return issue === undefined || issue === null ? undefined : issue.key;
}

/** The sentence of a change of one field: *Ben changed Priority from Normal to High*. */
export function changeSentence(entry: HistoryEntry): string {
  const who = whoOf(entry);
  const field = fieldLabel(entry.field ?? "");
  const before = sideOf(entry, "before");
  const after = sideOf(entry, "after");
  if (before !== undefined && after !== undefined) {
    return t("history.changed", { who, field, before, after });
  }
  if (after !== undefined) {
    return t("history.set", { who, field, after });
  }
  return before === undefined
    ? t("history.changed-long-text", { who, field })
    : t("history.cleared", { who, field });
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
