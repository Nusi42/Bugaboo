import { t } from "@bugaboo/messages";
import {
  Alert,
  alertTone,
  Button,
  buttonLook,
  Form,
  SubmitButton,
  Window,
  partOf,
  part,
} from "@bugaboo/ui";
import { useId } from "react";

import { messageOf } from "../../shared/errors.ts";
import { windowLabels } from "../../shared/window-labels.ts";
import { needsTheDescription, type OfferedChange } from "./ask-for.ts";
import { NewIssueField } from "./NewIssueField.tsx";
import type { IssueView } from "./slots.ts";
import { useAskFor } from "./use-ask-for.ts";
import type { IssueChanges } from "./use-issue-changes.ts";

export interface AskForProps {
  readonly issue: IssueView;
  /** The status change that needs something filled in; its button text is the title of the window. */
  readonly change: OfferedChange;
  readonly changes: IssueChanges;
  readonly onClose: () => void;
}

/**
 * *Ask for* (WF-04, UX §4.4): a small window with only the fields the status change needs. A field
 * that *Must be filled in* is named at its box when the server says it is still empty. The status
 * changes after *Save*, and *Undo* is on offer like for any change. The description is long text
 * with a lock of its own: a change that needs it shows a note that says to write it first, and
 * nothing to save.
 */
export function AskFor({ issue, change, changes, onClose }: AskForProps) {
  const form = useAskFor(issue, change, changes, onClose);
  const formId = useId();
  const blocked = needsTheDescription(change.needs);
  return (
    <Window
      isOpen
      onOpenChange={(open) => {
        if (!open) {
          onClose();
        }
      }}
      title={change.button}
      labels={windowLabels()}
      footer={
        <div {...partOf(part.windowFooterActions)}>
          <Button look={buttonLook.quiet} onPress={onClose}>
            {blocked ? t("common.close") : t("common.cancel")}
          </Button>
          {blocked ? null : (
            <SubmitButton form={formId} isPending={form.pending}>
              {change.button}
            </SubmitButton>
          )}
        </div>
      }
    >
      {blocked ? (
        <Alert tone={alertTone.warning}>{t("issues.ask-for-description")}</Alert>
      ) : (
        <Form id={formId} onSubmit={form.send}>
          <p {...partOf(part.muted)}>{t("issues.ask-for-intro", { to: change.to.name })}</p>
          {change.needs.map((need) => (
            <NewIssueField
              key={need.field}
              field={need.field}
              values={form.values}
              project={issue.project?.short_name ?? ""}
              problem={form.refusedField === need.field ? messageOf(form.refused) : undefined}
              onChange={form.fill}
            />
          ))}
          {form.refused !== undefined && form.refusedField === undefined ? (
            <Alert tone={alertTone.danger}>{messageOf(form.refused)}</Alert>
          ) : null}
        </Form>
      )}
    </Window>
  );
}
