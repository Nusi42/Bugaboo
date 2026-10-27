import type { SearchLanguage, Span, Suggestion } from "@bugaboo/wasm-search";

import { nowhere } from "./search-tree.ts";

/** The text and the place of the cursor after a suggestion was taken. */
export interface Edited {
  readonly text: string;
  readonly cursor: number;
}

/** The word a suggestion puts into the text; `null` for an offer that is a question for the server. */
export function wordOf(suggestion: Suggestion): string | null {
  return suggestion.type === "word" ? suggestion.text : null;
}

/**
 * The text with `word` in the place of what was typed of the word (`replace`). A space follows,
 * so the next word can be started, unless a bracket opens or the text goes on with a space.
 */
export function applyWord(text: string, replace: Span, word: string): Edited {
  const before = text.slice(0, replace.start);
  const after = text.slice(replace.end);
  const opens = word.endsWith("(");
  const spaced = opens || /^\s/u.test(after) ? word : `${word} `;
  return { text: `${before}${spaced}${after}`, cursor: before.length + spaced.length };
}

/** A name of a person or a group as it is written in a search: quoted where the language needs it. */
export function nameSource(language: SearchLanguage, name: string): string {
  return language.operandSource({ type: "text", text: name, span: nowhere });
}

/** An Issue ID as it is written in a search. */
export function issueSource(language: SearchLanguage, id: string): string {
  return language.operandSource({ type: "issue", id: { text: id, span: nowhere }, span: nowhere });
}
