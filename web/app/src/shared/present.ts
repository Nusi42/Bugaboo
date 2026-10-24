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

/**
 * Whether an item of a list holds every member of `names`: a list read with `fields=` has each member
 * optional, and an item that holds the members the whole thing always has is the whole thing. The
 * names come from the server's table (a generated list), so no screen writes them again.
 */
export function holdsAll<Item extends object, Name extends keyof Item>(
  item: Item,
  names: readonly Name[],
): item is Item & { readonly [Key in Name]-?: NonNullable<Item[Key]> } {
  return names.every((name) => item[name] !== undefined && item[name] !== null);
}
