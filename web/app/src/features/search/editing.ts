import type { Expr, Field, ReadableFields } from "@bugaboo/wasm-search";

import { fieldOf } from "./search-fields.ts";
import type { FieldFilter, FilterPlace } from "../../shared/search-tree.ts";

/** What is open for editing: a filter with a picker, or a filter shown as text. */
export type Editing =
  | {
      readonly kind: "field";
      readonly field: Field;
      readonly place?: FilterPlace;
      readonly filter?: FieldFilter;
    }
  | {
      readonly kind: "text";
      readonly place?: FilterPlace;
      readonly filter?: Expr;
    }
  | {
      /** An *Any of*: its filters, each picked like a filter of its own. */
      readonly kind: "any-of";
      readonly place?: FilterPlace;
      readonly members: readonly Expr[];
    };

/** What the window of a filter opens on when its button is pressed. */
export function editingFor(fields: ReadableFields, filter: Expr, place: FilterPlace): Editing {
  const field = filter.type === "field" ? fieldOf(fields, filter.field.word) : undefined;
  if (filter.type === "or") {
    return { kind: "any-of", place, members: filter.items };
  }
  return filter.type === "field" && field !== undefined
    ? { kind: "field", field, place, filter }
    : { kind: "text", place, filter };
}
