/**
 * How many characters a text has as the server counts them: Unicode code points, not the UTF-16
 * units `length` counts (an emoji is one character, not two).
 */
export function codePointsOf(text: string): number {
  return Array.from(text).length;
}

/** The first `count` characters of a text, cut between code points and never inside one. */
export function cutToCodePoints(text: string, count: number): string {
  return Array.from(text).slice(0, count).join("");
}
