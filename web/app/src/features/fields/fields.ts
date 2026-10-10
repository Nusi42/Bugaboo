import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { useQuery } from "@tanstack/react-query";

import { api } from "../../shared/api.ts";

/** An own field as the server shows it to this person (`FieldBody`). */
export type Field = Schema<"FieldBody">;
/** A choice of a *Pick one* or *Pick several* field. */
export type Choice = Schema<"FieldChoiceBody">;
/** What a kind of information allows. */
export type TypeInfo = Schema<"FieldTypeBody">;
/** A kind of information, by its word. */
export type KindWord = Schema<"FieldType">;
/** How a value is shown, by its word. */
export type LookWord = Schema<"Look">;

/** The look of plain text, which has no color or icon of its own. */
export const plainText: LookWord = "plain-text";

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
    select: (found: Field) => found,
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

/** The sentence under a kind of information: an example of what it holds. */
const examples: Readonly<Record<KindWord, () => string>> = {
  text: () => t("field-example.text"),
  "long-text": () => t("field-example.long-text"),
  "whole-number": () => t("field-example.whole-number"),
  decimal: () => t("field-example.decimal"),
  "yes-no": () => t("field-example.yes-no"),
  "web-address": () => t("field-example.web-address"),
  date: () => t("field-example.date"),
  "date-time": () => t("field-example.date-time"),
  duration: () => t("field-example.duration"),
  "pick-one": () => t("field-example.pick-one"),
  "pick-several": () => t("field-example.pick-several"),
  person: () => t("field-example.person"),
  people: () => t("field-example.people"),
  group: () => t("field-example.group"),
  issue: () => t("field-example.issue"),
};

/** An example of what a kind of information holds. */
export function typeExample(kind: KindWord): string {
  return examples[kind]();
}

/** Where a field is used, in words: the whole organization, or how many projects. */
export function scopeText(field: Field): string {
  const projects = field.scope.projects.length;
  if (field.scope.whole_organization) {
    return t("fields.scope-everywhere");
  }
  return t("fields.scope-projects", { count: projects });
}

/** The choice with this ID in a field. */
export function choiceOf(field: Field, id: string): Choice | undefined {
  return field.choices.find((choice) => choice.id === id);
}
