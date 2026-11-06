import {
  dataOf,
  fieldMembers,
  ownFieldPartPrefix,
  pageSizeMax,
  type Schema,
} from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import type { IssueBlockProps } from "../issues/index.ts";
import { useQuery } from "@tanstack/react-query";

import { api } from "../../shared/api.ts";
import { linesFrom } from "../../shared/paged.ts";
import { holdsAll } from "../../shared/present.ts";
import { queryKeys } from "../../shared/query-keys.ts";
import { detailsText } from "../../shared/list-text.ts";
import type { Picked } from "../../shared/reference.ts";

/** The values of the own fields of an item, by the key of the field, as the server sent them. */
export type FieldValues = Readonly<Record<string, unknown>>;

/** An own field as the server shows it to this person (`FieldBody`). */
export type OwnField = Schema<"FieldBody">;
/**
 * What a box needs of a field to read and word its value: its name, kind, help and whether it must
 * be filled in. An own field is one; a length of the issue itself (*Estimated time*, *Time left*)
 * is another, which is no own field.
 */
export type BoxField = Pick<OwnField, "name" | "kind"> & {
  /** The help under the box, in words; a length box has none of its own (it draws `LengthHint`). */
  readonly help?: OwnField["help"];
  readonly rules: Pick<OwnField["rules"], "required">;
};
/** A choice of a *Pick one* or *Pick several* field. */
export type Choice = Schema<"FieldChoiceBody">;
/** What a kind of information allows. */
export type TypeInfo = Schema<"FieldTypeBody">;
/** A kind of information, by its word. */
export type KindWord = Schema<"FieldType">;
/** How a value is shown, by its word. */
export type LookWord = Schema<"Look">;

/**
 * The value an editor sends: what the API takes for this field, or `null` to take the value away.
 * On the form of a new issue a person, group or issue picked is held with its name until the issue
 * is made.
 */
export type Sent =
  | string
  | number
  | boolean
  | readonly string[]
  | Picked
  | readonly Picked[]
  | null;

/** The name of the part of an issue that a long-text field is: what its lock and drafts are about. */
export function editPartOf(field: string): string {
  return [ownFieldPartPrefix, field].join("");
}

/** What a choice of a field is when it is dragged: only a choice can be dropped among the choices. */
export const choiceKind = "field-choice";

/** The look of plain text, which has no color or icon of its own. */
export const plainText = "plain-text" satisfies LookWord;

/** The look of a small colored label. */
export const badgeLook = "badge" satisfies LookWord;

/** The kinds of information a field can hold, with what each allows: the server's, never a second list. */
export function useFieldTypes() {
  return useQuery({
    ...api.query.queryOptions("get", "/api/v1/field-types"),
    select: (page: Schema<"FieldTypeListBody">) => page,
  });
}

/** The fields the person sees, retired ones too when asked. */
export function useFields(
  options: { readonly project?: string | undefined; readonly retired?: boolean } = {},
) {
  const query = {
    ...(options.project === undefined ? {} : { project: options.project }),
    ...(options.retired === true ? { retired: true } : {}),
  };
  return useQuery({
    queryKey: queryKeys.fields.list(query),
    queryFn: () => readAllFields(query),
  });
}

/** Reads the pages of the list of fields one after the other: a form needs every field. */
async function readAllFields(query: {
  readonly project?: string;
  readonly retired?: boolean;
}): Promise<{ readonly fields: readonly OwnField[] }> {
  return { fields: await readFieldPages(query, undefined, []) };
}

async function readFieldPages(
  query: { readonly project?: string; readonly retired?: boolean },
  cursor: string | undefined,
  before: readonly OwnField[],
): Promise<readonly OwnField[]> {
  const page = dataOf(
    await api.client.GET("/api/v1/fields", {
      params: {
        query: { ...query, limit: pageSizeMax, ...(cursor === undefined ? {} : { cursor }) },
      },
    }),
  );
  // A cursor is a position and a list may change between two requests: a field seen twice is kept
  // once, and a cursor that comes back ends the reading.
  const seen = new Set(before.map((each) => each.id));
  const fields = [...before, ...linesFrom([page], fieldOf).filter((each) => !seen.has(each.id))];
  const next = page.next_cursor;
  return next === undefined || next === null || next === cursor
    ? fields
    : readFieldPages(query, next, fields);
}

/** A field of the list, or nothing for an item that came without what a field always has. */
function fieldOf(item: Schema<"FieldItem">): OwnField | undefined {
  return holdsAll(item, fieldMembers) ? item : undefined;
}

/** One field. */
export function useField(id: string) {
  return useQuery({
    ...api.query.queryOptions("get", "/api/v1/fields/{field}", { params: { path: { field: id } } }),
    select: (found: OwnField) => found,
  });
}

/** Where a field is used now. */
export function useFieldUsage(id: string) {
  return useQuery({
    ...api.query.queryOptions("get", "/api/v1/fields/{field}/used-by", {
      params: { path: { field: id } },
    }),
    select: (usage: Schema<"FieldUsedByBody">) => usage,
  });
}

/** The name of a kind of information. */
export function typeName(kind: KindWord): string {
  return t(`field-type.${kind}`);
}

/** The name of a look. */
export function lookName(look: LookWord): string {
  return t(`field-look.${look}`);
}

/** An example of what a kind of information holds. */
export function typeExample(kind: KindWord): string {
  return t(`field-example.${kind}`);
}

/** Where a field is used, in words: the whole organization, or how many projects. */
export function scopeText(field: OwnField): string {
  const projects = field.scope.projects.length;
  if (field.scope.whole_organization) {
    return t("fields.scope-everywhere");
  }
  return t("fields.scope-projects", { count: projects });
}

/** What is special about a field, said in words next to its name: its kind, where it is used, retired. */
export function detailsOf(field: OwnField): string {
  return detailsText(
    [
      typeName(field.kind),
      t(`field-for.${field.applies_to}`),
      scopeText(field),
      field.retired ? t("fields.retired") : null,
    ].filter((word) => word !== null),
  );
}

/** The choice with this ID in a field. */
export function choiceOf(field: OwnField, id: string): Choice | undefined {
  return field.choices.find((choice) => choice.id === id);
}

/** Whether the choice has a number: a numbered choice stands where its number puts it, not where the buttons move it. */
export function isNumbered(choice: Choice): boolean {
  return choice.number !== undefined && choice.number !== null;
}

/** The choices with these IDs of a field, in the order the field lists them, found in the fields the browser has read. */
export function choicesHeldIn(
  fields: readonly OwnField[],
): (field: string, held: readonly string[]) => readonly Choice[] {
  return (field, held) =>
    fields
      .find((each) => each.id === field)
      ?.choices.filter((choice) => held.includes(choice.id)) ?? [];
}

/** The own fields made for `appliesTo` that reach an issue of this type: used in its project and its issue type, not retired. */
function reachingIssue(
  fields: readonly OwnField[],
  type: string | undefined,
  appliesTo: OwnField["applies_to"],
): readonly OwnField[] {
  return fields.filter(
    (field) =>
      field.applies_to === appliesTo &&
      !field.retired &&
      (field.scope.issue_types.length === 0 ||
        field.scope.issue_types.some((each) => each.id === type)),
  );
}

/** The own fields of an issue that the person sees: used in its project and its issue type. */
export function issueFieldsOf(
  fields: readonly OwnField[],
  issue: IssueBlockProps["issue"],
): readonly OwnField[] {
  return reachingIssue(fields, issue.issue_type?.id, "issues");
}

/**
 * The fields of time entries whose value an issue holds as the default for the time booked on it
 * (ADR-0035, decision 1): no fields of the issue, so its forms do not place them, and its details
 * show them in a group of their own.
 */
export function timeDefaultsOf(
  fields: readonly OwnField[],
  issue: IssueBlockProps["issue"],
): readonly OwnField[] {
  return timeEntryFieldsFor(fields, issue.issue_type?.id);
}

/**
 * The fields of time entries that reach an issue of this type (or any type): what the forms of time
 * and the defaults of the issue both start from.
 */
export function timeEntryFieldsFor(
  fields: readonly OwnField[],
  issueTypeId: string | undefined,
): readonly OwnField[] {
  return reachingIssue(fields, issueTypeId, "time-entries");
}
