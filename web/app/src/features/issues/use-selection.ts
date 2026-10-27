import { useMemo, useState } from "react";

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
  // The same object while the ticked issues are the same, so what is given it does not change on every draw.
  return useMemo(
    () => ({
      ids,
      has: (id: string) => ids.includes(id),
      set: (id: string, selected: boolean) => {
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
    }),
    [ids],
  );
}
