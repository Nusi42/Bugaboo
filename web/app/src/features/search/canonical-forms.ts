import type { ReadableFields, SearchLanguage } from "@bugaboo/wasm-search";
import { useMemo } from "react";

import { useSearchFields } from "./search-fields.ts";
import { useSearchLanguage } from "./search-language.ts";

/**
 * How the words of a stored search are written now (ADR-0005 amendment M5, decision 11): the one
 * rule is the language module's (`wordKey`, `check`), so a search or a list saved before D3 reads
 * like one saved today.
 */
export interface CanonicalForms {
  /** The canonical text of a stored search text; the text itself when it does not check. */
  readonly text: (stored: string) => string;
  /** The word of a field that the person may read for a word that spells it the old way; any other word stays. */
  readonly word: (word: string) => string;
}

/** The canonical forms for these fields: the one rule is the language module's. */
export function canonicalFormsOf(language: SearchLanguage, fields: ReadableFields): CanonicalForms {
  const words = fields.fields.map((field) => field.word);
  return {
    text: (stored) => language.check(stored, fields).canonical ?? stored,
    word: (word) => {
      const key = language.wordKey(word);
      return words.find((each) => language.wordKey(each) === key) ?? word;
    },
  };
}

/** The canonical forms, once the language and the readable fields are there; none before. */
export function useCanonicalForms(within?: string): CanonicalForms | undefined {
  const language = useSearchLanguage();
  const fields = useSearchFields(within).data?.fields;
  return useMemo(
    () =>
      language === undefined || fields === undefined
        ? undefined
        : canonicalFormsOf(language, fields),
    [language, fields],
  );
}
