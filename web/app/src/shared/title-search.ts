import type { SearchLanguage } from "@bugaboo/wasm-search";

/** The word of the field whose text quick search looks in: only the title, not the long texts. */
const TITLE_FIELD = "title";

/** No place in a text: the printer writes the text from the words, and the places mean nothing to it. */
const nowhere = { start: 0, end: 0 } as const;

/**
 * The search `title contains "<words>"` as the search language writes it, so a quote in the words
 * cannot become part of the language, and the last word is a start while it is being typed. Nothing
 * when the text holds no word.
 */
export function titleSearchText(
  language: Pick<SearchLanguage, "print" | "openLastWord">,
  words: string,
): string | undefined {
  if (words.trim() === "") {
    return undefined;
  }
  // The last word is still being typed unless a space follows it: it finds the words that start
  // with it, as far as the language lets a start be short.
  const text = language.openLastWord(words.trimStart()).trimEnd();
  return language.print({
    filter: {
      type: "field",
      field: { word: TITLE_FIELD, span: nowhere },
      test: { type: "contains", text: { text, span: nowhere }, span: nowhere },
      span: nowhere,
    },
    sort: [],
    span: nowhere,
  });
}
