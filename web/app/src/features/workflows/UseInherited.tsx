import { t } from "@bugaboo/messages";
import { Alert, alertTone, buttonLook, ConfirmDialog } from "@bugaboo/ui";

import { codeOf, messageOf } from "../../shared/errors.ts";
import { personName } from "../../shared/person.ts";
import { useEditingNow } from "./use-editing-now.ts";
import { useDeleteWorkflow, type WorkflowEditor } from "./workflows.ts";

/**
 * *Use the inherited workflow again*: takes the workflow of this place away, after a question.
 * While somebody else is editing it the server refuses, and the page says who it is (as the person
 * may see them).
 */
export function UseInherited({ slot }: { readonly slot: NonNullable<WorkflowEditor["workflow"]> }) {
  const remove = useDeleteWorkflow();
  const busy = codeOf(remove.error) === "edit.in_progress";
  const editing = useEditingNow(slot.id, busy);
  return (
    <>
      <ConfirmDialog
        triggerLabel={t("workflows.use-inherited")}
        triggerName={t("workflows.use-inherited")}
        title={t("workflows.use-inherited-title")}
        text={t("workflows.use-inherited-text")}
        confirmLabel={t("workflows.use-inherited")}
        cancelLabel={t("common.cancel")}
        confirmLook={buttonLook.danger}
        onConfirm={() => {
          remove.mutate({ workflow: slot.id, version: slot.version });
        }}
      />
      {remove.isError ? (
        <Alert tone={alertTone.danger}>
          {busy && editing.data !== null && editing.data !== undefined
            ? t("workflows.delete-held", { name: personName(editing.data.holder) })
            : messageOf(remove.error)}
        </Alert>
      ) : null}
    </>
  );
}
