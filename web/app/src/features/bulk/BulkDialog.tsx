import { t } from "@bugaboo/messages";
import { Alert, alertTone, AskDialog, Button } from "@bugaboo/ui";

import { messageOf } from "../../shared/errors.ts";
import { actionName } from "./action-names.ts";
import { ActionFields } from "./ActionFields.tsx";
import type { BulkApplied, BulkKind } from "./bulk.ts";
import { BulkPreviewView } from "./BulkPreviewView.tsx";
import { useBulkDialog } from "./use-bulk-dialog.ts";

export interface BulkDialogProps {
  readonly kind: BulkKind;
  /** The IDs of the issues that are ticked. */
  readonly issues: readonly string[];
  readonly isOpen: boolean;
  readonly onOpenChange: (open: boolean) => void;
  readonly onApplied: (applied: BulkApplied) => void;
}

/**
 * The dialog of changing many issues at once (ISS-09): fill in the thing, *Show what changes*, and
 * only then *Apply*. The preview is made from the versions of the issues, and applying sends those
 * versions back, so an issue that changed meanwhile is skipped and not overwritten.
 */
export function BulkDialog({ kind, issues, isOpen, onOpenChange, onApplied }: BulkDialogProps) {
  const dialog = useBulkDialog(kind, issues, onApplied);
  const { preview } = dialog;
  return (
    <AskDialog
      isOpen={isOpen}
      onOpenChange={(open) => {
        if (!open) {
          dialog.forget();
        }
        onOpenChange(open);
      }}
      title={t("bulk.title", { action: actionName(kind), count: issues.length })}
      text={t("bulk.bar")}
      confirmLabel={t("bulk.apply")}
      cancelLabel={t("common.cancel")}
      confirmDisabled={!dialog.canApply}
      onConfirm={dialog.send}
    >
      <ActionFields
        kind={kind}
        draft={dialog.draft}
        relations={dialog.relations}
        onDraft={dialog.fill}
      />
      <div>
        <Button
          isDisabled={dialog.action === null}
          isPending={preview.isPending}
          onPress={dialog.ask}
        >
          {t("bulk.show-changes")}
        </Button>
      </div>
      {preview.isError ? <Alert tone={alertTone.danger}>{messageOf(preview.error)}</Alert> : null}
      {preview.data === undefined ? null : (
        <BulkPreviewView preview={preview.data} plain={dialog.plain} />
      )}
    </AskDialog>
  );
}
