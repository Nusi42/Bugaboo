/** Which of the two texts a paragraph is taken from. */
export type Side = "mine" | "theirs";

/** One place in both texts: the paragraph of each, and whether they are the same. */
export interface CompareRow {
  readonly mine: string;
  readonly theirs: string;
  readonly same: boolean;
}

/** The paragraphs of a text: what stands between empty lines, without empty ones at the ends. */
export function paragraphsOf(text: string): string[] {
  return text
    .replaceAll("\r\n", "\n")
    .split(/\n{2,}/u)
    .map((paragraph) => paragraph.trim())
    .filter((paragraph) => paragraph !== "");
}

/**
 * The two texts side by side, paragraph by paragraph (*Compare and save*, ISS-17): the first
 * paragraph of each, the second of each, and so on. A paragraph one text has and the other lacks
 * stands against an empty place.
 */
export function compareRows(mine: string, theirs: string): CompareRow[] {
  const own = paragraphsOf(mine);
  const other = paragraphsOf(theirs);
  return Array.from({ length: Math.max(own.length, other.length) }, (_, at) => {
    const [a, b] = [own[at] ?? "", other[at] ?? ""];
    return { mine: a, theirs: b, same: a === b };
  });
}

/** The text that follows from a choice for each row that differs: the others are the same on both sides. */
export function combine(rows: readonly CompareRow[], picks: readonly Side[]): string {
  return rows
    .map((row, at) => (row.same || picks[at] !== "theirs" ? row.mine : row.theirs))
    .filter((paragraph) => paragraph !== "")
    .join("\n\n");
}
