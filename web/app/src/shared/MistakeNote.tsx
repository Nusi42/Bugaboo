import { part, partOf } from "@bugaboo/ui";

import { mistakeText, type Mistakes } from "./mistakes.ts";

/** Why a form sent as a whole is held: a box holds something it cannot take yet. Nothing while none does. */
export function MistakeNote({ mistakes }: { readonly mistakes: Mistakes }) {
  return mistakes.length === 0 ? null : <p {...partOf(part.fieldError)}>{mistakeText()}</p>;
}
