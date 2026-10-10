import { priorityWords, statusWords, type Schema } from "@bugaboo/api-client";
import { t, type MessageKey } from "@bugaboo/messages";

import { namedPerson, type NamedPerson } from "../../shared/person.ts";
import { known } from "../../shared/present.ts";
import type { IssueDetail } from "./slots.ts";

/** The short values of an issue a person changes in place. The description is long text: it has a lock. */
export interface IssueValues {
  readonly title?: string;
  readonly status?: Schema<"Status">;
  readonly priority?: Schema<"Priority">;
  /** The people it is assigned to from now on: all of them. */
  readonly assignedTo?: readonly NamedPerson[];
}

/** One change on its way: the values, and the version on screen that it is made on. */
export interface IssueEdit {
  readonly values: IssueValues;
  readonly version: number;
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

/** The values of the same fields as they are now: what *Undo* sends. */
export function valuesBefore(detail: IssueDetail, values: IssueValues): IssueValues {
  const { issue } = detail;
  const [title, status, priority] = [
    known(issue.title),
    known(issue.status),
    known(issue.priority),
  ];
  const people = (issue.assigned_to ?? []).flatMap((person) => namedPerson(person) ?? []);
  return {
    ...(values.title === undefined || title === undefined ? {} : { title }),
    ...(values.status === undefined || status === undefined ? {} : { status }),
    ...(values.priority === undefined || priority === undefined ? {} : { priority }),
    ...(values.assignedTo === undefined ? {} : { assignedTo: people }),
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
          names: values.assignedTo.map((person) => person.name).join(", "),
        }),
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
  return {
    ...(typeof title === "string" ? { title } : {}),
    ...(status === undefined ? {} : { status }),
    ...(priority === undefined ? {} : { priority }),
  };
}

/** The word of a list that a value is, or nothing. */
function wordOf<Word extends string>(words: readonly Word[], value: unknown): Word | undefined {
  return words.find((word) => word === value);
}
