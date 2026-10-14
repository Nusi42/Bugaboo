import type { OwnField } from "./fields.ts";

/** What makes a line start afresh: the field, the value it holds and the version of the item. */
export function lineKey(field: OwnField, value: unknown, version = 0): string {
  return [field.id, JSON.stringify(value), version].join("|");
}
