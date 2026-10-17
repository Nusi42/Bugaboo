import { editedKind, RemoveEditable } from "../../shared/editing/index.ts";
import { useDeleteWorkflow, whatOf, type WorkflowEditor } from "./workflows.ts";

/** *Use the inherited workflow again*: takes the workflow of this place away, after a question. */
export function UseInherited({ slot }: { readonly slot: NonNullable<WorkflowEditor["workflow"]> }) {
  const remove = useDeleteWorkflow();
  return (
    <RemoveEditable
      what={whatOf(slot.id)}
      kind={editedKind.workflow}
      error={remove.error}
      onConfirm={() => {
        remove.mutate({ workflow: slot.id, version: slot.version });
      }}
    />
  );
}
