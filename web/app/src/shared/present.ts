/**
 * A value of an answer that is there, or nothing. The server leaves a field out when the reader may
 * not see it and sends `null` for one that is empty; the screens treat both as "not there".
 */
export function known<Value>(value: Value | null | undefined): Value | undefined {
  return value ?? undefined;
}

/** The word of a list that a value is, or nothing. */
export function wordOf<Word extends string>(
  words: readonly Word[],
  value: unknown,
): Word | undefined {
  return words.find((word) => word === value);
}
