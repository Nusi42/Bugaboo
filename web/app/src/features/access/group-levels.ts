/** One field or kind with its level and the sentences that say why. */
export interface LevelEntry {
  /** What the field or kind is called. */
  readonly name: string;
  readonly level: string;
  readonly sentences: readonly string[];
}

/** Fields or kinds that have the same level for the same reason. */
export interface LevelGroup {
  readonly names: readonly string[];
  readonly level: string;
  readonly sentences: readonly string[];
}

/**
 * Groups the entries that have the same level for the same reason (so the reason is read once),
 * the biggest group first, the others in the order their first member comes.
 */
export function groupLevels(entries: readonly LevelEntry[]): readonly LevelGroup[] {
  const groups = new Map<string, LevelGroup>();
  for (const entry of entries) {
    const key = JSON.stringify([entry.level, entry.sentences]);
    const group = groups.get(key);
    groups.set(key, {
      names: [...(group?.names ?? []), entry.name],
      level: entry.level,
      sentences: entry.sentences,
    });
  }
  return [...groups.values()].toSorted((a, b) => b.names.length - a.names.length);
}
