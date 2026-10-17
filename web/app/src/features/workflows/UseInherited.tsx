import { t } from "@bugaboo/messages";

import { RemoveEditable, type RemoveWords } from "../../shared/editing/index.ts";
import { useDeleteWorkflow, whatOf, type WorkflowEditor } from "./workflows.ts";

/** What the question and the notes about taking a workflow away say. */
function wordsOfWorkflow(): RemoveWords {
  return {
    button: t("workflows.use-inherited"),
    title: t("workflows.use-inherited-title"),
    text: t("workflows.use-inherited-text"),
    held: (name) => t("workflows.delete-held", { name }),
  };
}

/** *Use the inherited workflow again*: takes the workflow of this place away, after a question. */
export function UseInherited({ slot }: { readonly slot: NonNullable<WorkflowEditor["workflow"]> }) {
  const remove = useDeleteWorkflow();
  return (
    <RemoveEditable
      what={whatOf(slot.id)}
      words={wordsOfWorkflow()}
      error={remove.error}
      onConfirm={() => {
        remove.mutate({ workflow: slot.id, version: slot.version });
      }}
    />
  );
}
