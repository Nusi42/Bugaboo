import { inPlace, scrollBoxOf, type ColorToken } from "@bugaboo/ui";

/** The ID of the row of a color on the page, which *Show* of a pair that is hard to read goes to. */
export function colorRowId(token: ColorToken): string {
  return `theme-color-${token}`;
}

/** The field of a color in its row: the text field, not the picker of the device. */
const TYPED_FIELD = "input:not([type='color'])";

/** Opens the group of a color, brings its row into sight and puts the cursor in its field. */
export function focusColor(token: ColorToken): void {
  const row = document.querySelector(`#${colorRowId(token)}`);
  const group = row?.closest("details");
  if (group !== null && group !== undefined) {
    group.open = true;
  }
  row?.scrollIntoView({ block: "center" });
  row?.querySelector<HTMLInputElement>(TYPED_FIELD)?.focus(inPlace);
}

/** The ID of the readability on top of the editor: a refused save brings the cursor there. */
export const READABILITY_ID = "theme-readability";

/**
 * Brings the readability into sight with the colors it names under it, and the cursor to it. Only the
 * box the editor scrolls in moves, to where the readability starts under the head of the pane: the
 * main column of a computer never scrolls (UX §6.5).
 */
export function focusReadability(): void {
  const found = document.querySelector<HTMLElement>(`#${READABILITY_ID}`);
  // The band of the verdict sticks on top; the groups it heads start where the band and the list of
  // the colors start in the flow.
  const start = found?.parentElement?.parentElement ?? null;
  const box = scrollBoxOf(start);
  if (start !== null && box !== null) {
    const under = Number(getComputedStyle(box).scrollPaddingTop.replace(/px$/u, "")) || 0;
    const above = start.getBoundingClientRect().top - box.getBoundingClientRect().top - under;
    box.scrollBy({ top: Math.min(0, above) });
  }
  found?.focus(inPlace);
}
