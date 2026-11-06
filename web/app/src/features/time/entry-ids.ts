/** The ID of the item of an entry in its list, so the focus can come back to its button. */
export function entryItemId(entry: string): string {
  return `entry-${entry}`;
}
