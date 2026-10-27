import { dataOf, type Schema } from "@bugaboo/api-client";
import type { SearchLanguage } from "@bugaboo/wasm-search";
import { useQuery } from "@tanstack/react-query";

import { api } from "../../shared/api.ts";
import { ownValueText } from "../../shared/own-value-text.ts";
import { linesFrom, pageSize } from "../../shared/paged.ts";
import { known } from "../../shared/present.ts";
import { queryKeys } from "../../shared/query-keys.ts";

/** A field of the organization as a column of the list needs it. */
export interface OwnColumn {
  /** The ID of the field, which is its key in the values of an issue. */
  readonly id: string;
  readonly kind: Schema<"FieldType">;
  /** The names of its choices, by their IDs. */
  readonly choices: Readonly<Record<string, string>>;
}

/** The fields of the organization by their search word, which is made from their name by one rule of the language. */
export type OwnColumns = Readonly<Record<string, OwnColumn>>;

/** The names of the choices of a field by their IDs. */
function choicesOf(
  choices: readonly Schema<"FieldChoiceBody">[] | null | undefined,
): Readonly<Record<string, string>> {
  return Object.fromEntries((choices ?? []).map((choice) => [choice.id, choice.name]));
}

/** The column a field stands for, by its search word; nothing for a field that came without what a column needs. */
function ownColumnOf(language: SearchLanguage | undefined) {
  return (
    field: Schema<"FieldItem">,
  ): { readonly word: string; readonly column: OwnColumn } | undefined => {
    const [id, name, kind] = [known(field.id), known(field.name), known(field.kind)];
    return language === undefined || id === undefined || name === undefined || kind === undefined
      ? undefined
      : {
          word: language.ownFieldWord(name),
          column: { id, kind, choices: choicesOf(field.choices) },
        };
  };
}

/**
 * The fields the person sees, read when a column needs one: an issue carries the value of a field
 * by the ID of the field, and a column is named by the word of the search language.
 */
export function useOwnColumns(language: SearchLanguage | undefined, needed: boolean) {
  const fields = useQuery({
    queryKey: queryKeys.fields.list({ limit: pageSize.largest }),
    enabled: needed,
    queryFn: async () =>
      dataOf(
        await api.client.GET("/api/v1/fields", { params: { query: { limit: pageSize.largest } } }),
      ),
  });
  return Object.fromEntries(
    linesFrom([fields.data ?? { items: [] }], ownColumnOf(language)).map(({ word, column }) => [
      word,
      column,
    ]),
  );
}

/** The words of a value of a field of the organization as it stands in a row: a choice by its name, the rest as the details word it. */
export function ownColumnText(column: OwnColumn | undefined, value: unknown): string | undefined {
  if (column === undefined || value === undefined || value === null) {
    return undefined;
  }
  if (typeof value === "string" && value in column.choices) {
    return column.choices[value];
  }
  if (Array.isArray(value)) {
    return value
      .map((each: unknown) => ownColumnText(column, each))
      .filter((each) => each !== undefined)
      .join(", ");
  }
  return ownValueText(column.kind, value);
}
