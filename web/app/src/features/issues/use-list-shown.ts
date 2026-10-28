import { useCallback, useState } from "react";

/** An issue a list draws: its ID, and its readable ID as the ID column shows it. */
export interface ShownIssue {
  readonly id: string;
  readonly key: string;
}

/** The issues the groups of a list draw, by group: what the count line and *Tick the issues shown* act on. */
export interface ListShown {
  /** The IDs of the issues drawn now, once each. */
  readonly ids: readonly string[];
  /** How many characters the longest readable ID of the issues drawn now has: the ID column is as wide. */
  readonly keyChars: number;
  /** A group (or the whole list) tells which issues it draws. */
  readonly report: (group: string, issues: readonly ShownIssue[]) => void;
}

/** Keeps what each group of a list draws. */
export function useListShown(): ListShown {
  const [shown, setShown] = useState<Readonly<Record<string, readonly ShownIssue[]>>>({});
  const report = useCallback((group: string, issues: readonly ShownIssue[]) => {
    setShown((before) => ({ ...before, [group]: issues }));
  }, []);
  const all = Object.values(shown).flat();
  return {
    ids: [...new Set(all.map((issue) => issue.id))],
    keyChars: Math.max(0, ...all.map((issue) => issue.key.length)),
    report,
  };
}
