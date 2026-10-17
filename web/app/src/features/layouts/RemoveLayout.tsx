import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";

import { RemoveEditable, type RemoveWords } from "../../shared/editing/index.ts";
import { useDeleteLayout, whatOf } from "./layouts.ts";

/** What the question and the notes about taking a layout away say. */
function wordsOfLayout(): RemoveWords {
  return {
    button: t("layouts.use-inherited"),
    title: t("layouts.use-inherited-title"),
    text: t("layouts.use-inherited-text"),
    held: (name) => t("layouts.delete-held", { name }),
  };
}

/** *Reset to default*: takes the layout of this place away, after a question. */
export function RemoveLayout({
  slot,
}: {
  readonly slot: NonNullable<Schema<"LayoutEditorBody">["layout"]>;
}) {
  const remove = useDeleteLayout();
  return (
    <RemoveEditable
      what={whatOf(slot.id)}
      words={wordsOfLayout()}
      error={remove.error}
      onConfirm={() => {
        remove.mutate({ layout: slot.id, version: slot.version });
      }}
    />
  );
}
