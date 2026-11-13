import { hashKey, type QueryKey } from "@tanstack/react-query";
import { useState } from "react";

/** What a read that is shown says: its key, its answer and why it was refused. */
export interface ShownRead {
  readonly queryKey: QueryKey;
  readonly data: unknown;
  readonly error: unknown;
}

/** The refusal kept for one read, by the key of the read. */
export interface Kept {
  readonly key: string;
  readonly error: unknown;
}

/**
 * The refusal to keep after a read is drawn, and the refusal to show. A read with an answer keeps no
 * refusal of its own (a refusal of it shows while it lasts); a read without one keeps its refusal,
 * for its own key only, so the note stays while it is read again and never shows for another read.
 * Applied again to its own answer it gives the same: the state settles at once.
 */
export function keptAfter(
  kept: Kept | null,
  { queryKey, data, error }: ShownRead,
): { readonly kept: Kept | null; readonly shown: unknown } {
  const key = hashKey(queryKey);
  const known = error !== null && error !== undefined;
  if (data !== undefined) {
    return { kept: null, shown: known ? error : null };
  }
  if (known) {
    return {
      kept: kept?.key === key && kept.error === error ? kept : { key, error },
      shown: error,
    };
  }
  return { kept, shown: kept?.key === key ? kept.error : null };
}

/**
 * Why a read was refused, kept while it is read again (UX §6.4). The query cache forgets the refusal
 * of a read that has no answer the moment it reads it again, so the note would go and come back on
 * every read in the background; this keeps it until the read answers, for the same key only.
 */
export function useKeptError(read: ShownRead): unknown {
  const [kept, setKept] = useState<Kept | null>(null);
  const next = keptAfter(kept, read);
  // Kept as the read changes, while it is drawn (React's way of state that follows a value).
  if (next.kept !== kept) {
    setKept(next.kept);
  }
  return next.shown;
}
