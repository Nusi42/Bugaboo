import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Alert, alertTone, buttonLook, ConfirmDialog } from "@bugaboo/ui";

import { codeOf, messageOf } from "../../shared/errors.ts";
import { personName } from "../../shared/person.ts";
import { useDeleteLayout, useEditingNow } from "./layouts.ts";

/**
 * *Use the inherited layout again*: takes the layout of this place away, after a question. While
 * somebody else is editing it the server refuses, and the page says who it is (as the person may
 * see them).
 */
export function RemoveLayout({
  slot,
}: {
  readonly slot: NonNullable<Schema<"LayoutEditorBody">["layout"]>;
}) {
  const remove = useDeleteLayout();
  const busy = codeOf(remove.error) === "edit.in_progress";
  const editing = useEditingNow(slot.id, busy);
  return (
    <>
      <ConfirmDialog
        triggerLabel={t("layouts.use-inherited")}
        triggerName={t("layouts.use-inherited")}
        title={t("layouts.use-inherited-title")}
        text={t("layouts.use-inherited-text")}
        confirmLabel={t("layouts.use-inherited")}
        cancelLabel={t("common.cancel")}
        confirmLook={buttonLook.danger}
        onConfirm={() => {
          remove.mutate({ layout: slot.id, version: slot.version });
        }}
      />
      {remove.isError ? (
        <Alert tone={alertTone.danger}>
          {busy && editing.data !== null && editing.data !== undefined
            ? t("layouts.delete-held", {
                name: personName(editing.data.holder),
              })
            : messageOf(remove.error)}
        </Alert>
      ) : null}
    </>
  );
}
