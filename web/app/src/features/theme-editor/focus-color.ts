import type { ColorToken } from "@bugaboo/ui";

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
  row?.querySelector<HTMLInputElement>(TYPED_FIELD)?.focus();
}
