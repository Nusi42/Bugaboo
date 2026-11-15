import type { ReactNode } from "react";

import { partOf } from "./parts.ts";
import { part } from "./tokens.ts";

/**
 * Keeps the room of what it holds, unseen and out of reach of the keyboard, the pointer and screen readers: the place of
 * a part whose first answer is on its way, so it comes in without narrowing what stands beside it (UX §6.4). It is the
 * part a hold is (`ReadingHold`), kept for good: no first read ends it.
 */
export function KeptRoom({ children }: { readonly children: ReactNode }) {
  return (
    <span {...partOf(part.readingHold, { states: ["kept"] })} aria-hidden inert>
      {children}
    </span>
  );
}
