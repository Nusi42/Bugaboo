const gaps = /(\s+)/u;

/** The words of a text and the white space between them, in order; no piece is empty. */
export function wordsAndGaps(text: string): readonly string[] {
  return text.split(gaps).filter((piece) => piece !== "");
}

/** Whether a piece of [`wordsAndGaps`] is white space and not a word. */
export function isGap(piece: string): boolean {
  return /^\s/u.test(piece);
}

/** The words of a text without the white space between them, in order; no word is empty. */
export function wordsOnly(text: string): readonly string[] {
  return wordsAndGaps(text).filter((piece) => !isGap(piece));
}

/** Whether every word of the query is in the text, in any order and in any case (a search box). */
export function containsEveryWord(text: string, query: string): boolean {
  const lower = text.toLowerCase();
  return wordsOnly(query.toLowerCase()).every((word) => lower.includes(word));
}
