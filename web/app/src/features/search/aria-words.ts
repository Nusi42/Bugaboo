/** Words of the HTML and ARIA standards for the box and the list of offers, not text for people. */
export const ariaWords = {
  listbox: "listbox",
  option: "option",
  list: "list",
  offerIndex: "data-offer",
} as const;

/** The ID of the offer at `index` in the list `list`, which the box names with `aria-activedescendant`. */
export function offerId(list: string, index: number): string {
  return `${list}-${String(index)}`;
}
