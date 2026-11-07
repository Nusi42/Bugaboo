/** The list with an item taken in (`on`) or out, the item once at most and the others as they were. */
export function toggled<Item>(list: readonly Item[], item: Item, on: boolean): readonly Item[] {
  const without = list.filter((one) => one !== item);
  return on ? [...without, item] : without;
}
