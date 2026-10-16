import { priorityWords, statusWords, type Schema } from "@bugaboo/api-client";
import { t, type MessageKey } from "@bugaboo/messages";

import { listText } from "../../shared/list-text.ts";
import { namedPerson, type NamedPerson } from "../../shared/person.ts";
import { known, wordOf } from "../../shared/present.ts";
import type { IssueDetail } from "./slots.ts";

/** The short values of an issue a person changes in place. The description is long text: it has a lock. */
export interface IssueValues {
  readonly title?: string;
  readonly status?: Schema<"Status">;
  readonly priority?: Schema<"Priority">;
  /** The people it is assigned to from now on: all of them. */
  readonly assignedTo?: readonly NamedPerson[];
  /** Values of own fields (WP-05) as the API takes them, by the key of the field; `null` takes one away. */
  readonly own?: Readonly<Record<string, unknown>>;
}

/**
 * The name of a row of the Details that saves in place with *Saved* and *Undo* in its own row (R7):
 * the word of a built-in field (`priority`, `assigned-to`) or the ID of an own field. The title and
 * the status say it elsewhere.
 */
export type RowField = string;

/** The row of the Details a change belongs to, when it belongs to one. */
export function rowFieldOf(values: IssueValues): RowField | undefined {
  if (values.assignedTo !== undefined) {
    return "assigned-to";
  }
  if (values.priority !== undefined) {
    return "priority";
  }
  // One own field changed: its row says it. Several at once have no one row.
  const own = Object.keys(values.own ?? {});
  return own.length === 1 ? own[0] : undefined;
}

/** One change on its way: the values, and the version on screen that it is made on. */
export interface IssueEdit {
  readonly values: IssueValues;
  readonly version: number;
  /** Called with the version the server made of it, once it did: what *Undo* of this change is sent on. */
  readonly landed?: (version: number) => void;
}

/** Whether what the cache holds under the key is the page of an issue. */
export function isDetail(value: unknown): value is IssueDetail {
  return typeof value === "object" && value !== null && "issue" in value && "may_change" in value;
}

/** A person as the server shows them to somebody who may see names, for the page before it answers. */
function shownByName(person: NamedPerson): Schema<"PersonRef"> {
  return { shown: "name", id: person.id, name: person.name };
}

/**
 * The page of the issue as it looks after a change, before the server has answered. Contract with
 * `change_issue` on the server: every change counts the version up by exactly one, so the next
 * change on screen is made on that one.
 */
export function after(before: IssueDetail | undefined, edit: IssueEdit): IssueDetail | undefined {
  if (before === undefined) {
    return undefined;
  }
  const { values } = edit;
  const issue = {
    ...before.issue,
    ...(values.title === undefined ? {} : { title: values.title }),
    ...(values.status === undefined ? {} : { status: values.status }),
    ...(values.priority === undefined ? {} : { priority: values.priority }),
    ...(values.assignedTo === undefined ? {} : { assigned_to: values.assignedTo.map(shownByName) }),
    version: edit.version + 1,
  };
  return { ...before, issue };
}

/**
 * A value of an own field as the API takes it, from the way the page shows it: text, numbers and
 * yes or no as they are, a list of them, and a person, group or issue by its ID. `undefined` for a
 * value that cannot be sent back (a person shown by role only).
 */
function sendable(shown: unknown): unknown {
  if (shown === undefined || shown === null) {
    return null;
  }
  if (typeof shown === "string" || typeof shown === "number" || typeof shown === "boolean") {
    return shown;
  }
  if (Array.isArray(shown)) {
    const items: readonly unknown[] = shown;
    const sent = items.map((one) => sendable(one));
    return sent.includes(undefined) ? undefined : sent;
  }
  if (typeof shown !== "object") {
    return undefined;
  }
  const id: unknown = Reflect.get(shown, "id");
  return typeof id === "string" ? id : undefined;
}

/** The values of the own fields in a change as they are now, for *Undo*; none when one cannot be sent back. */
function ownBefore(detail: IssueDetail, own: Readonly<Record<string, unknown>>) {
  const now = detail.issue.own_fields ?? {};
  const before = Object.keys(own).map((key) => [key, sendable(Reflect.get(now, key))] as const);
  return before.some(([, value]) => value === undefined) ? undefined : Object.fromEntries(before);
}

/** The values of the same fields as they are now: what *Undo* sends. */
export function valuesBefore(detail: IssueDetail, values: IssueValues): IssueValues {
  const { issue } = detail;
  const [title, status, priority] = [
    known(issue.title),
    known(issue.status),
    known(issue.priority),
  ];
  const people = (issue.assigned_to ?? []).flatMap((person) => namedPerson(person) ?? []);
  const own = values.own === undefined ? undefined : ownBefore(detail, values.own);
  return {
    ...(values.title === undefined || title === undefined ? {} : { title }),
    ...(values.status === undefined || status === undefined ? {} : { status }),
    ...(values.priority === undefined || priority === undefined ? {} : { priority }),
    ...(values.assignedTo === undefined ? {} : { assignedTo: people }),
    ...(own === undefined ? {} : { own }),
  };
}

/** A change of one field in a sentence: "Priority changed to High." */
function said(field: MessageKey, value: string): string {
  return t("issues.changed", { field: t(field), value });
}

/** What a change says in a sentence, for *Undo*: "Priority changed to High." */
export function changeSentence(values: IssueValues): string {
  const parts: (string | undefined)[] = [
    values.title === undefined ? undefined : said("issue-field.title", values.title),
    values.status === undefined
      ? undefined
      : said("issue-field.status", t(`status.${values.status}`)),
    values.priority === undefined
      ? undefined
      : said("issue-field.priority", t(`priority.${values.priority}`)),
    values.assignedTo === undefined
      ? undefined
      : t("issues.changed-people", {
          field: t("issue-field.assigned-to"),
          count: values.assignedTo.length,
          names: listText(values.assignedTo.map((person) => person.name)),
        }),
    values.own === undefined
      ? undefined
      : t("issues.changed-own", { count: Object.keys(values.own).length }),
  ];
  return parts.filter((part): part is string => part !== undefined).join(" ");
}

/**
 * The short values in a change that a block of another package asks for (`IssueBlockProps.onChange`):
 * the ones the page knows are taken, the others are left to the package that made the block.
 */
export function valuesOf(change: Readonly<Record<string, unknown>>): IssueValues {
  const title = change["title"];
  const status = wordOf(statusWords, change["status"]);
  const priority = wordOf(priorityWords, change["priority"]);
  const own = change["fields"];
  return {
    ...(typeof own === "object" && own !== null && !Array.isArray(own)
      ? { own: Object.fromEntries(Object.entries(own)) }
      : {}),
    ...(typeof title === "string" ? { title } : {}),
    ...(status === undefined ? {} : { status }),
    ...(priority === undefined ? {} : { priority }),
  };
}
