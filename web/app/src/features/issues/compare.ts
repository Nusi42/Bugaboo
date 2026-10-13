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

/** A stretch of a paragraph, and whether the other text lacks it. */
export interface WordPart {
  readonly text: string;
  readonly changed: boolean;
}

/** The length of the common run of words that starts at each pair of places (the table of a longest common run). */
function commonRuns(
  words: readonly string[],
  other: readonly string[],
): readonly (readonly number[])[] {
  const rows: number[][] = [Array.from({ length: other.length + 1 }, () => 0)];
  for (let from = words.length - 1; from >= 0; from -= 1) {
    const below = rows[0] ?? [];
    const row = [...below];
    for (let at = other.length - 1; at >= 0; at -= 1) {
      row[at] =
        words[from] === other[at]
          ? 1 + (below[at + 1] ?? 0)
          : Math.max(below[at] ?? 0, row[at + 1] ?? 0);
    }
    rows.unshift(row);
  }
  return rows;
}

/** Whether each word of `words` is left out of the longest run of words that `other` has in the same order. */
function unmatchedWords(words: readonly string[], other: readonly string[]): readonly boolean[] {
  const runs = commonRuns(words, other);
  const run = (from: number, at: number): number => runs[from]?.[at] ?? 0;
  const unmatched = words.map(() => true);
  let from = 0;
  let at = 0;
  while (from < words.length && at < other.length) {
    if (words[from] === other[at]) {
      unmatched[from] = false;
      from += 1;
      at += 1;
    } else if (run(from + 1, at) >= run(from, at + 1)) {
      from += 1;
    } else {
      at += 1;
    }
  }
  return unmatched;
}

/** Whether a token is white space. */
function isSpace(token: string): boolean {
  return /^\s/u.test(token);
}

/** The words of a text and the white space between them, in order. */
function tokensOf(text: string): readonly string[] {
  return text.split(/(\s+)/u).filter((token) => token !== "");
}

/**
 * For each token of a text, whether it is a word the other text does not have, or null for white
 * space.
 */
function changedTokens(tokens: readonly string[], other: string): readonly (boolean | null)[] {
  const unmatched = unmatchedWords(
    tokens.filter((token) => !isSpace(token)),
    tokensOf(other).filter((token) => !isSpace(token)),
  );
  let word = -1;
  return tokens.map((token) => {
    if (isSpace(token)) {
      return null;
    }
    word += 1;
    return unmatched[word] === true;
  });
}

/**
 * A paragraph in stretches, with the words that the other text does not have marked as changed
 * (*Show changes*, M3-F54). The white space between two marked words belongs to the mark, so a
 * changed phrase is one stretch; the stretches put together are the paragraph.
 */
export function wordParts(text: string, other: string): readonly WordPart[] {
  const tokens = tokensOf(text);
  const changedAt = changedTokens(tokens, other);
  const parts: WordPart[] = [];
  tokens.forEach((token, place) => {
    const changed =
      changedAt[place] ?? (changedAt[place - 1] === true && changedAt[place + 1] === true);
    const last = parts.at(-1);
    if (last?.changed === changed) {
      parts[parts.length - 1] = { text: last.text + token, changed };
    } else {
      parts.push({ text: token, changed });
    }
  });
  return parts;
}
