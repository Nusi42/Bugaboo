import type { SearchLanguage } from "@bugaboo/wasm-search";

/** The word of the field whose text quick search looks in: only the title, not the long texts. */
const TITLE_FIELD = "title";

/** No place in a text: the printer writes the text from the words, and the places mean nothing to it. */
const nowhere = { start: 0, end: 0 } as const;

/**
 * The search `title contains "<words>"` as the search language writes it, so a quote in the words
 * cannot become part of the language. Nothing when the text holds no word.
 */
export function titleSearchText(
  language: Pick<SearchLanguage, "print">,
  words: string,
): string | undefined {
  const text = words.trim();
  if (text === "") {
    return undefined;
  }
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
