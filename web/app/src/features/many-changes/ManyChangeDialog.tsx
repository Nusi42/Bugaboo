import { t } from "@bugaboo/messages";
import { Alert, alertTone, AskDialog, Button } from "@bugaboo/ui";

import { messageOf } from "../../shared/errors.ts";
import { WordingNotice } from "../../shared/with-wording.tsx";
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

/** *Show what changes*: the preview is asked for once the thing to change is filled in. */
function ShowChanges({
  disabled,
  pending,
  onAsk,
}: {
  readonly disabled: boolean;
  readonly pending: boolean;
  readonly onAsk: () => void;
}) {
  return (
    <div>
      <Button isDisabled={disabled} isPending={pending} onPress={onAsk}>
        {t("many-changes.show-changes")}
      </Button>
    </div>
  );
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
  // The button names what it does: the issues the preview says will change, not every ticked one.
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
      confirmLabel={applyName(kind, preview.data?.summary.will_change ?? issues.length)}
      cancelLabel={t("common.cancel")}
      confirmDisabled={!dialog.canApply}
      onConfirm={dialog.send}
    >
      <ActionFields
        kind={kind}
        draft={dialog.draft}
        relations={dialog.relations}
        statusChanges={dialog.statusChanges}
        onDraft={dialog.fill}
      />
      <ShowChanges
        disabled={dialog.action === null}
        pending={preview.isPending}
        onAsk={dialog.ask}
      />
      {preview.isError ? <Alert tone={alertTone.danger}>{messageOf(preview.error)}</Alert> : null}
      {preview.data === undefined ? null : <WordingNotice />}
      {preview.data === undefined ? null : (
        <>
          <WordingNotice />
          <ManyChangePreviewView preview={preview.data} plain={dialog.plain} />
        </>
      )}
    </AskDialog>
  );
}
