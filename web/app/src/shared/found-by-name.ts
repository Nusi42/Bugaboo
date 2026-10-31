/** The things of a list whose name holds the words typed into its search box, without regard to capitals. */
export function foundByName<Thing extends { readonly name: string }>(
  things: readonly Thing[],
  words: string,
): readonly Thing[] {
  const wanted = words.trim().toLowerCase();
  return things.filter((thing) => thing.name.toLowerCase().includes(wanted));
}
