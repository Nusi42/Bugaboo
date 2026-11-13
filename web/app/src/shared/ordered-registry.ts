/** Something an area adds to another area's list: named by its `id`, placed by its `order`. */
export interface Ordered {
  /** Names the entry in a test and in the order. */
  readonly id: string;
  /** Entries are drawn in this order, then by `id`. */
  readonly order: number;
}

/** A list that other areas add to when the app starts (an extension point), kept in its order. */
export interface OrderedRegistry<Entry extends Ordered> {
  /** Adds an entry; one with the same `id` replaces the old one. */
  readonly add: (entry: Entry) => void;
  /** The entries in their order. */
  readonly all: () => readonly Entry[];
}

/** A new, empty list of entries that other areas add to. The one place that keeps such a list in its order. */
export function orderedRegistry<Entry extends Ordered>(): OrderedRegistry<Entry> {
  let entries: readonly Entry[] = [];
  return {
    add: (entry) => {
      const others = entries.filter((known) => known.id !== entry.id);
      entries = [...others, entry].toSorted(
        (a, b) => a.order - b.order || a.id.localeCompare(b.id),
      );
    },
    all: () => entries,
  };
}
