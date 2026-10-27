import { dataOf, type Schema } from "@bugaboo/api-client";
import type { Field, FieldKind, Operator, ReadableFields } from "@bugaboo/wasm-search";
import { keepPreviousData, useQuery } from "@tanstack/react-query";

import { api } from "../../shared/api.ts";
import { queryKeys } from "../../shared/query-keys.ts";

/**
 * Every kind of value and every operator the module knows, as a record: the compiler stops this
 * file when the types of the module get a new one, so a word the server sends can be checked
 * against the types instead of being believed.
 */
const kinds: Readonly<Record<FieldKind, true>> = {
  text: true,
  number: true,
  duration: true,
  date: true,
  choice: true,
  person: true,
  project: true,
  issue: true,
};

const operators: Readonly<Record<Operator, true>> = {
  is: true,
  is_not: true,
  less: true,
  greater: true,
  less_or_equal: true,
  greater_or_equal: true,
  in: true,
  contains: true,
  is_empty: true,
  is_not_empty: true,
  in_last: true,
  member_of: true,
  within: true,
};

function isKind(word: string): word is FieldKind {
  return Object.hasOwn(kinds, word);
}

function isOperator(word: string): word is Operator {
  return Object.hasOwn(operators, word);
}

/**
 * The fields the server sent as the checker of the module wants them. A field of a kind this app
 * does not know is left out (a newer server): a search by it is then an unknown field, which is
 * safe, and the rest keeps working.
 */
export function readableFieldsOf(body: Schema<"SearchFieldsBody">): ReadableFields {
  const fields: Field[] = [];
  for (const field of body.fields) {
    if (isKind(field.kind)) {
      fields.push({
        word: field.word,
        kind: field.kind,
        operators: field.operators.filter(isOperator),
        choices: field.choices,
      });
    }
  }
  return { fields };
}

/**
 * The fields the person may search by, the same the checker of the server gets (QRY-05), for a
 * search that looks in a project (`within`) or everywhere.
 */
export function useSearchFields(within?: string) {
  return useQuery({
    queryKey: queryKeys.search.fields(within),
    placeholderData: keepPreviousData,
    queryFn: async () =>
      readableFieldsOf(
        dataOf(
          await api.client.GET("/api/v1/search/fields", {
            params: { query: within === undefined ? {} : { within } },
          }),
        ),
      ),
  });
}

/** The field with this word, if the person may search by it. */
export function fieldOf(fields: ReadableFields, word: string): Field | undefined {
  return fields.fields.find((field) => field.word === word);
}
