import { useCallback, useState } from "react";

/** The issues the groups of a list draw, by group: what the count line and *Tick the issues shown* act on. */
export interface ListShown {
  /** The IDs of the issues drawn now, once each. */
  readonly ids: readonly string[];
  /** A group (or the whole list) tells which issues it draws. */
  readonly report: (group: string, issues: readonly string[]) => void;
}

/** Keeps what each group of a list draws. */
export function useListShown(): ListShown {
  const [shown, setShown] = useState<Readonly<Record<string, readonly string[]>>>({});
  const report = useCallback((group: string, issues: readonly string[]) => {
    setShown((before) => ({ ...before, [group]: issues }));
  }, []);
  return { ids: [...new Set(Object.values(shown).flat())], report };
}
