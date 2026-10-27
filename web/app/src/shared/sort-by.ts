import type { Direction, SearchLanguage } from "@bugaboo/wasm-search";

import { nowhere } from "./search-tree.ts";

/**
 * The search text with its `sort by` changed to one key (a click on a column name). The module's
 * parser reads the text and its printer writes it, so the filters stay as they are. A text that
 * does not parse is returned as it was. `direction` null is the usual one of the field.
 */
export function withSortBy(
  language: SearchLanguage,
  text: string,
  field: string,
  direction: Direction | null,
): string {
  const { search } = language.parse(text);
  if (search === null) {
    return text;
  }
  return language.print({
    ...search,
    sort: [{ field: { word: field, span: nowhere }, direction, span: nowhere }],
  });
}
