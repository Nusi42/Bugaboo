import type { Schema } from "@bugaboo/api-client";
import { fieldWords } from "@bugaboo/api-client";

/** The word of a built-in field, as the layout names it. */
export type FieldWord = Schema<"Field">;

/** Where a group of rows of the Details goes, among the fields of the layout. */
export type Places =
  /** The group draws these built-in fields: it stands where the first of them stands. */
  | readonly FieldWord[]
  /** The group draws the own fields: it stands where the first own field of the layout stands. */
  | "own-fields"
  /** The group is not a field of the layout (*Followers*): it comes after the rest. */
  | "end";

/** A group of rows of the Details, and what it draws (a node with a key of its own). */
export interface RowGroup<Node> {
  readonly id: string;
  readonly places: Places;
  readonly node: Node;
}

/** The place of a group in the layout, as the position of its first field; none counts as last. */
function rankOf(layout: readonly string[], places: Places): number {
  const found = layout.findIndex((key) =>
    places === "own-fields"
      ? !(fieldWords as readonly string[]).includes(key)
      : places !== "end" && (places as readonly string[]).includes(key),
  );
  return found === -1 ? Number.POSITIVE_INFINITY : found;
}

/**
 * The groups of rows in the order the layout puts their fields (CFG-01, M3-F102): the layout is
 * the one that the form uses (`FormView.sections`), so there is no second order. Groups the layout
 * does not place, and the groups before the layout is read, keep the order they came in.
 */
export function inLayoutOrder<Node>(
  layout: readonly string[],
  groups: readonly RowGroup<Node>[],
): Node[] {
  return groups
    .toSorted((a, b) => {
      const [first, second] = [rankOf(layout, a.places), rankOf(layout, b.places)];
      if (first === second) {
        return 0;
      }
      return first < second ? -1 : 1;
    })
    .map((group) => group.node);
}
