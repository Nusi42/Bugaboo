const gaps = /(\s+)/u;

/** The words of a text and the white space between them, in order; no piece is empty. */
export function wordsAndGaps(text: string): readonly string[] {
  return text.split(gaps).filter((piece) => piece !== "");
}

/** Whether a piece of [`wordsAndGaps`] is white space and not a word. */
export function isGap(piece: string): boolean {
  return /^\s/u.test(piece);
}
