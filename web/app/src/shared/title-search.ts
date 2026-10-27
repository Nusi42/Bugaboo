import type { SearchLanguage } from "@bugaboo/wasm-search";

import { listWord } from "./list-columns.ts";
import { fieldFilter, nowhere } from "./search-tree.ts";

/** The word of the field whose text the finders look in: only the title, not the long texts. The word is the list's (`listWord`). */
export const titleWord = listWord.title;

/**
 * The search `title contains "<words>"` as the search language writes it, so a quote in the words
 * cannot become part of the language. The last word is a start while it is being typed (`logi`
 * finds `login`), as far as the language lets a start be short. Nothing when the text holds no word.
 */
export function titleSearchText(
  language: Pick<SearchLanguage, "print" | "openLastWord">,
  words: string,
): string | undefined {
  if (words.trim() === "") {
    return undefined;
  }
  // The last word is still being typed unless a space follows it.
  const text = language.openLastWord(words.trimStart()).trimEnd();
  return language.print({
    filter: fieldFilter(titleWord, {
      type: "contains",
      text: { text, span: nowhere },
      span: nowhere,
    }),
    sort: [],
    span: nowhere,
  });
}
