import { t } from "@bugaboo/messages";
import { Alert, alertTone, AskDialog, Button } from "@bugaboo/ui";

import { messageOf } from "../../shared/errors.ts";
import { actionName, applyName } from "./action-names.ts";
import { ActionFields } from "./ActionFields.tsx";
import type { ManyChangeApplied, ManyChangeKind } from "./many-changes.ts";
import { ManyChangePreviewView } from "./ManyChangePreviewView.tsx";
import { useManyChangeDialog } from "./use-many-change-dialog.ts";

export interface ManyChangeDialogProps {
  readonly kind: ManyChangeKind;
  /** The project whose issues are ticked: its short name. */
  readonly project: string;
  /** The IDs of the issues that are ticked. */
  readonly issues: readonly string[];
  readonly isOpen: boolean;
  readonly onOpenChange: (open: boolean) => void;
  readonly onApplied: (applied: ManyChangeApplied) => void;
}

/**
 * The dialog of changing many issues at once (ISS-09): fill in the thing, *Show what changes*, and
 * only then *Apply*. The preview is made from the versions of the issues, and applying sends those
 * versions back, so an issue that changed meanwhile is skipped and not overwritten.
 */
export function ManyChangeDialog({
  kind,
  project,
  issues,
  isOpen,
  onOpenChange,
  onApplied,
}: ManyChangeDialogProps) {
  const dialog = useManyChangeDialog(kind, project, issues, onApplied);
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
      title={t("many-changes.title", { action: actionName(kind), count: issues.length })}
      text={t("many-changes.dialog-text")}
      confirmLabel={applyName(kind, issues.length)}
      cancelLabel={t("common.cancel")}
      confirmDisabled={!dialog.canApply}
      onConfirm={dialog.send}
    >
      <ActionFields
        kind={kind}
        draft={dialog.draft}
        relations={dialog.relations}
        statuses={dialog.statuses}
        onDraft={dialog.fill}
      />
      <div>
        <Button
          isDisabled={dialog.action === null}
          isPending={preview.isPending}
          onPress={dialog.ask}
        >
          {t("many-changes.show-changes")}
        </Button>
      </div>
      {preview.isError ? <Alert tone={alertTone.danger}>{messageOf(preview.error)}</Alert> : null}
      {preview.data === undefined ? null : (
        <ManyChangePreviewView preview={preview.data} plain={dialog.plain} />
      )}
    </AskDialog>
  );
}
