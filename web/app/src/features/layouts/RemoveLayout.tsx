import type { Schema } from "@bugaboo/api-client";

import { editedKind, RemoveEditable } from "../../shared/editing/index.ts";
import { useDeleteLayout, whatOf } from "./layouts.ts";

/** *Use the inherited layout again*: takes the layout of this place away, after a question. */
export function RemoveLayout({
  slot,
}: {
  readonly slot: NonNullable<Schema<"LayoutEditorBody">["layout"]>;
}) {
  const remove = useDeleteLayout();
  return (
    <RemoveEditable
      what={whatOf(slot.id)}
      kind={editedKind.layout}
      error={remove.error}
      onConfirm={() => {
        remove.mutate({ layout: slot.id, version: slot.version });
      }}
    />
  );
}
