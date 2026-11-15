import { conflictOf, type Conflict } from "@bugaboo/api-client";

import { messageOf } from "./errors.ts";

/** Why a change was refused: the reason in words, or the change somebody made first (a stale version). */
export type Refused = { readonly reason: string } | { readonly conflict: Conflict };

/** What a refusal of a change was: a change made first, with its version, or anything else in words. */
export function refusedOf(error: unknown): Refused {
  const conflict = conflictOf(error);
  return conflict === undefined ? { reason: messageOf(error) } : { conflict };
}
