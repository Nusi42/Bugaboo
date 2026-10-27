import type { SearchLanguage } from "@bugaboo/wasm-search";

import { addFilter, fieldFilter, nowhere } from "./search-tree.ts";

/** What a filter asks of a field: a value by its shown name (ADR-0005 item 9), or that the field is empty. */
export type FilterValue = string | { readonly empty: true };

/**
 * The search text with one more filter, `<field> is <value>` or `<field> is empty`, added through
 * the parser and the printer of the search language, so the value is quoted as the language
 * needs it. A text that does not parse is returned as it was.
 */
export function withFilter(
  language: SearchLanguage,
  text: string,
  field: string,
  value: FilterValue,
): string {
  const { search } = language.parse(text);
  if (search === null) {
    return text;
  }
  const test =
    typeof value === "string"
      ? {
          type: "is" as const,
          value: { type: "text" as const, text: value, span: nowhere },
          span: nowhere,
        }
      : { type: "is_empty" as const, span: nowhere };
  return language.print(addFilter(search, fieldFilter(field, test)));
}
