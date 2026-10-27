import { useState } from "react";

/** The issues ticked in a list, by their IDs: what the bar for changing many at once (WP-08) acts on. */
export interface IssueSelection {
  /** The IDs of the ticked issues, in the order they were ticked. */
  readonly ids: readonly string[];
  readonly has: (id: string) => boolean;
  readonly set: (id: string, selected: boolean) => void;
  /** Ticks exactly these issues (*Tick all results*). */
  readonly replace: (ids: readonly string[]) => void;
  readonly clear: () => void;
}

/** The selection of a list. */
export function useSelection(): IssueSelection {
  const [ids, setIds] = useState<readonly string[]>([]);
  return {
    ids,
    has: (id) => ids.includes(id),
    set: (id, selected) => {
      setIds(
        selected
          ? [...ids.filter((known) => known !== id), id]
          : ids.filter((known) => known !== id),
      );
    },
    replace: setIds,
    clear: () => {
      setIds([]);
    },
  };
}
