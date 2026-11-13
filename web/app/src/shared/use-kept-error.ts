import { hashKey, type QueryKey } from "@tanstack/react-query";
import { useState } from "react";

/** What a read that is shown says: its key, its answer and why it was refused. */
export interface ShownRead {
  readonly queryKey: QueryKey;
  readonly data: unknown;
  readonly error: unknown;
}

/**
 * Why a read was refused, kept while it is read again (UX §6.4). The query cache forgets the refusal
 * of a read that has no answer the moment it reads it again, so the note would go and come back on
 * every read in the background; this keeps it until the read answers, for the same key only.
 */
export function useKeptError({ queryKey, data, error }: ShownRead): unknown {
  const key = hashKey(queryKey);
  const [kept, setKept] = useState<{ readonly key: string; readonly error: unknown } | null>(null);
  // Kept as the read changes, while it is drawn (React's way of state that follows a value).
  // A read with an answer keeps no refusal of its own: a refusal of it shows while it lasts (and
  // keeping it there would make the state go back and forth without end).
  const known = error !== null && error !== undefined;
  if (data !== undefined) {
    if (kept !== null) {
      setKept(null);
    }
  } else if (known && (kept?.key !== key || kept.error !== error)) {
    setKept({ key, error });
  }
  if (known || data !== undefined) {
    return known ? error : null;
  }
  return kept?.key === key ? kept.error : null;
}
