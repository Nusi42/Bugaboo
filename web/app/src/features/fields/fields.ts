import { ownFieldPartPrefix, type Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import type { IssueBlockProps } from "../issues/index.ts";
import { useQuery } from "@tanstack/react-query";

import { api } from "../../shared/api.ts";

/** The values of the own fields of an item, by the key of the field, as the server sent them. */
export type FieldValues = Readonly<Record<string, unknown>>;

/** An own field as the server shows it to this person (`FieldBody`). */
export type OwnField = Schema<"FieldBody">;
/** A choice of a *Pick one* or *Pick several* field. */
export type Choice = Schema<"FieldChoiceBody">;
/** What a kind of information allows. */
export type TypeInfo = Schema<"FieldTypeBody">;
/** A kind of information, by its word. */
export type KindWord = Schema<"FieldType">;
/** How a value is shown, by its word. */
export type LookWord = Schema<"Look">;

/** The name of the part of an issue that a long-text field is: what its lock and drafts are about. */
export function editPartOf(field: string): string {
  return [ownFieldPartPrefix, field].join("");
}

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
  return useQuery({
    ...api.query.queryOptions("get", "/api/v1/fields", {
      params: {
        query: {
          ...(options.project === undefined ? {} : { project: options.project }),
          ...(options.retired === true ? { retired: true } : {}),
        },
      },
    }),
    select: (page: Schema<"FieldListBody">) => page,
  });
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

/** The choice with this ID in a field. */
export function choiceOf(field: OwnField, id: string): Choice | undefined {
  return field.choices.find((choice) => choice.id === id);
}

/** The own fields of an issue that the person sees: used in its project and its issue type. */
export function issueFieldsOf(
  fields: readonly OwnField[],
  issue: IssueBlockProps["issue"],
): readonly OwnField[] {
  const type = issue.issue_type?.id;
  return fields.filter(
    (field) =>
      field.applies_to === "issues" &&
      (field.scope.issue_types.length === 0 ||
        field.scope.issue_types.some((each) => each.id === type)),
  );
}
