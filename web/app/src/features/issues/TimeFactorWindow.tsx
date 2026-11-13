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

import { useWindowProps } from "../../shared/use-window-props.ts";
import { windowForms } from "../../shared/window-forms.ts";
import type { IssueView } from "./slots.ts";
import { otherPersonName, timeFactorTitle, type TimeFactor } from "./time-factor.ts";
import { TimeFactorFields } from "./TimeFactorFields.tsx";
import { useTimeFactorDraft } from "./use-time-factor-draft.ts";

export interface TimeFactorWindowProps {
  /** The issue: its *Estimated time*, when the reader may read it. */
  readonly issue: Pick<IssueView, "estimated_time">;
  readonly readableId: string;
  readonly factor: TimeFactor;
  /** The factor the reader is shown now, in percent. */
  readonly percent: number;
  /** Whether the reader may read *Estimated time*: only then can the factor be given as a length. */
  readonly mayReadEstimate: boolean;
  readonly onClose: () => void;
}

/**
 * *How long will it take you?* (PLAN-14, UX §4.4): the person on the bar, or somebody who may change
 * the field, gives the time factor as a length (*Takes me 10h*), which the server works out against
 * *Estimated time*, or as a factor (×1.25), and sees the sum as it is typed. For somebody else the
 * window asks *How long will it take Lea?* and says that Lea will be told. The server decides whether
 * the factor may be set and what it comes to.
 */
export function TimeFactorWindow(props: TimeFactorWindowProps) {
  const { issue, readableId, factor, percent, mayReadEstimate, onClose } = props;
  const formId = useId();
  const windowProps = useWindowProps(windowForms.timeFactor);
  const estimate = mayReadEstimate ? (issue.estimated_time ?? undefined) : undefined;
  const draft = useTimeFactorDraft(readableId, factor, percent, estimate !== undefined);
  const otherName = otherPersonName(factor);
  return (
    <Window
      {...windowProps}
      title={timeFactorTitle(factor)}
      isOpen
      onOpenChange={(isOpen) => {
        if (!isOpen) {
          onClose();
        }
      }}
      footer={
        <div {...partOf(part.windowFooterActions)}>
          <Button look={buttonLook.quiet} onPress={onClose}>
            {t("common.cancel")}
          </Button>
          <SubmitButton form={formId}>{t("issues.time-factor-save")}</SubmitButton>
        </div>
      }
    >
      <Form
        id={formId}
        onSubmit={() => {
          void draft.save().then((saved) => {
            if (saved) {
              onClose();
            }
          });
        }}
      >
        {otherName === undefined ? null : (
          <p {...partOf(part.text)}>{t("issues.time-factor-told", { name: otherName })}</p>
        )}
        {draft.problem === null ? null : <Alert tone={alertTone.danger}>{draft.problem}</Alert>}
        <TimeFactorFields draft={draft} estimate={estimate} otherName={otherName} />
      </Form>
    </Window>
  );
}
