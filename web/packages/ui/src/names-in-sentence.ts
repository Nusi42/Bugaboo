// Where a name stands in a sentence: as a whole name, never as a piece of a longer one.

const NAME_CHARACTER = /^[\p{L}\p{N}_-]$/u;
const LETTER_OR_DIGIT = /^[\p{L}\p{N}]$/u;

/** The characters of a text between two places: what stands right next to a value. */
function charactersAt(sentence: string, from: number, to: number): string[] {
  return Array.from(sentence.slice(Math.max(from, 0), to));
}

/** Whether the value stands as a whole name in the sentence from `start`: not a piece of a longer name. */
function standsAlone(sentence: string, start: number, end: number): boolean {
  const [before, twoBefore] = charactersAt(sentence, start - 4, start).toReversed();
  const [after, twoAfter] = charactersAt(sentence, end, end + 4);
  const insideBefore =
    (before !== undefined && NAME_CHARACTER.test(before)) ||
    (before === "." && twoBefore !== undefined && LETTER_OR_DIGIT.test(twoBefore));
  const insideAfter =
    (after !== undefined && NAME_CHARACTER.test(after)) ||
    (after === "." && twoAfter !== undefined && LETTER_OR_DIGIT.test(twoAfter));
  return !insideBefore && !insideAfter;
}

/** Where the value stands as a whole name in the sentence, each place by its start. */
export function wholeNamePlaces(sentence: string, value: string): readonly number[] {
  // An empty value stands nowhere (and `indexOf("")` finds it at every place, without end).
  if (value === "") {
    return [];
  }
  const places: number[] = [];
  for (let at = sentence.indexOf(value); at !== -1; at = sentence.indexOf(value, at + 1)) {
    if (standsAlone(sentence, at, at + value.length)) {
      places.push(at);
    }
  }
  return places;
}

/**
 * The longest of the names that the sentence holds as a whole name, if it holds one: `a.txt` is not
 * in a sentence about `data.txt`, and of `data.txt` and `a.txt` the sentence about both is drawn
 * with the longer.
 */
export function longestNameIn(sentence: string, names: readonly string[]): string | undefined {
  return names
    .filter((name) => wholeNamePlaces(sentence, name).length > 0)
    .toSorted((a, b) => b.length - a.length)
    .at(0);
}
