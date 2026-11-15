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
 * box the editor scrolls in moves (the main column of a computer never scrolls, UX §6.5), back until
 * what follows the band of the verdict stands under it with the gap of the groups: the band sticks on
 * top, and never covers what it names.
 */
export function focusReadability(): void {
  const found = document.querySelector<HTMLElement>(`#${READABILITY_ID}`);
  const band = found?.parentElement ?? null;
  const next = band?.nextElementSibling ?? null;
  const box = scrollBoxOf(band);
  if (band !== null && next !== null && box !== null) {
    const gap =
      Number(getComputedStyle(band.parentElement ?? band).rowGap.replace(/px$/u, "")) || 0;
    const under = next.getBoundingClientRect().top - band.getBoundingClientRect().bottom - gap;
    box.scrollBy({ top: Math.min(0, under) });
  }
  found?.focus(inPlace);
}
