import { manyChangesIssuesMax } from "@bugaboo/api-client";

/** The most issues that can be ticked at once with *Tick all results* (M5-Q9): what one change may name. */
export const MAX_TICKED = manyChangesIssuesMax;

/** What the bar offers beside the rows that are ticked. */
export type TickAllOffer =
  | { readonly kind: "none" }
  | { readonly kind: "all"; readonly count: number }
  | { readonly kind: "narrow"; readonly limit: number };

/** What to offer when `shown` rows are drawn and `total` issues match. */
export function tickAllOffer({
  shown,
  total,
}: {
  readonly shown: number;
  readonly total: number;
}): TickAllOffer {
  if (total <= shown) {
    return { kind: "none" };
  }
  return total > MAX_TICKED ? { kind: "narrow", limit: MAX_TICKED } : { kind: "all", count: total };
}
