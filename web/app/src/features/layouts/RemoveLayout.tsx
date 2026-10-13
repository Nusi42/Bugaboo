import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Alert, alertTone, buttonLook, ConfirmDialog } from "@bugaboo/ui";

import { messageOf } from "../../shared/errors.ts";
import { useDeleteLayout } from "./layouts.ts";

/** *Use the inherited layout again*: takes the layout of this place away, after a question. */
export function RemoveLayout({
  slot,
}: {
  readonly slot: NonNullable<Schema<"LayoutEditorBody">["layout"]>;
}) {
  const remove = useDeleteLayout();
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
      {remove.isError ? <Alert tone={alertTone.danger}>{messageOf(remove.error)}</Alert> : null}
    </>
  );
}
