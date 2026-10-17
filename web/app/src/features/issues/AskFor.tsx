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
import { askKeyOf, type OfferedChange } from "./ask-for.ts";
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
 * with a lock of its own, so a change that needs it says to write it first.
 */
export function AskFor({ issue, change, changes, onClose }: AskForProps) {
  const form = useAskFor(issue, change, changes, onClose);
  const formId = useId();
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
            {t("common.cancel")}
          </Button>
          <SubmitButton form={formId} isPending={form.pending}>
            {t("common.save")}
          </SubmitButton>
        </div>
      }
    >
      <Form id={formId} onSubmit={form.send}>
        <p {...partOf(part.muted)}>{t("issues.ask-for-intro", { to: change.to.name })}</p>
        {form.asked.map((each) =>
          each.kind === "description" ? (
            <Alert key={each.kind} tone={alertTone.warning}>
              {t("issues.ask-for-description")}
            </Alert>
          ) : (
            <NewIssueField
              key={askKeyOf(each)}
              field={askKeyOf(each)}
              values={form.values}
              project={issue.project?.short_name ?? ""}
              problem={form.refusedField === askKeyOf(each) ? messageOf(form.refused) : undefined}
              onChange={form.fill}
            />
          ),
        )}
        {form.refused !== undefined && form.refusedField === undefined ? (
          <Alert tone={alertTone.danger}>{messageOf(form.refused)}</Alert>
        ) : null}
      </Form>
    </Window>
  );
}
