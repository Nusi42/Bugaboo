import type { Mistake } from "@bugaboo/wasm-search";

/** A mistake of the end of a text that is still being typed: something is still missing. */
export function isUnfinished(mistake: Mistake, unfinishedAt: number | undefined): boolean {
  return unfinishedAt !== undefined && mistake.span.start >= unfinishedAt;
}

/**
 * What of the mistakes is a fault and what only a hint: while the box has the focus, what is wrong
 * at the end of the text is a hint (something is still missing), the rest are faults.
 */
export function faultsOf(
  mistakes: readonly Mistake[],
  focused: boolean,
  textLength: number,
): { readonly unfinishedAt: number | undefined; readonly faults: readonly Mistake[] } {
  const unfinishedAt = focused ? textLength : undefined;
  return {
    unfinishedAt,
    faults: mistakes.filter((mistake) => !isUnfinished(mistake, unfinishedAt)),
  };
}
