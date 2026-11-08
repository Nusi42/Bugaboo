import { t } from "@bugaboo/messages";
import {
  Alert,
  alertTone,
  fieldKind,
  Form,
  SubmitButton,
  TextField,
  partOf,
  part,
  useLayout,
} from "@bugaboo/ui";
import { useId, useState } from "react";

import { messageOf } from "../../shared/errors.ts";
import { limits } from "./assistant.ts";
import { codePointsOf } from "./code-points.ts";

interface AskRowProps {
  readonly phone: boolean;
  readonly isPending: boolean;
  readonly isDisabled: boolean;
  /** Nothing is typed yet: the button waits and says why. */
  readonly isEmpty: boolean;
}

/** The button that sends the question, with the reason when it waits for something to be typed. */
function AskRow({ phone, isPending, isDisabled, isEmpty }: AskRowProps) {
  const reasonId = useId();
  const button = (
    <SubmitButton
      isPending={isPending}
      isDisabled={isDisabled || isEmpty}
      wide={phone}
      {...(isEmpty ? { describedBy: reasonId } : {})}
    >
      {t("assistant.ask")}
    </SubmitButton>
  );
  return (
    <div {...partOf(part.row)}>
      {isEmpty ? (
        <span id={reasonId} {...partOf(part.muted)}>
          {t("assistant.ask-empty")}
        </span>
      ) : null}
      {/* On a computer the button that sends stands at the end (R6); on a phone it fills the width. */}
      {phone ? button : <span {...partOf(part.rowEnd)}>{button}</span>}
    </div>
  );
}

export interface AskBoxProps {
  /** Sends the question. */
  readonly onAsk: (text: string) => void;
  readonly isPending: boolean;
  /** AI does not count here: nothing can be asked. */
  readonly isDisabled: boolean;
  readonly error: unknown;
  /** The box is emptied when this changes: a question was answered. */
  readonly answeredCount: number;
}

/**
 * The box to ask, with its button. The most a question may hold is told under the box; a longer one is
 * refused with a sentence and never cut, so what is sent is what the person wrote.
 */
export function AskBox({ onAsk, isPending, isDisabled, error, answeredCount }: AskBoxProps) {
  const [typed, setTyped] = useState({ text: "", after: answeredCount });
  const phone = useLayout() === "phone";
  // A question that was answered leaves the box empty; one that failed stays, to be sent again.
  const text = typed.after === answeredCount ? typed.text : "";
  const tooLong = codePointsOf(text.trim()) > limits.question;
  // Nothing typed yet: the button waits, and says why.
  const empty = text.trim().length === 0 && !isDisabled;
  const send = (
    <AskRow
      phone={phone}
      isPending={isPending}
      isDisabled={isDisabled || tooLong}
      isEmpty={empty}
    />
  );
  return (
    <Form
      onSubmit={() => {
        if (text.trim().length > 0 && !tooLong) {
          onAsk(text.trim());
        }
      }}
    >
      <div {...partOf(part.assistantAsk)}>
        {isPending ? <Alert>{t("assistant.working")}</Alert> : null}
        {error === null || error === undefined ? null : (
          <Alert tone={alertTone.danger}>{messageOf(error)}</Alert>
        )}
        <TextField
          label={t("assistant.question")}
          description={t("assistant.question-limit", { count: limits.question })}
          errorMessage={
            tooLong ? t("assistant.question-too-long", { count: limits.question }) : undefined
          }
          kind={fieldKind.multiline}
          value={text}
          onChange={(next) => {
            setTyped({ text: next, after: answeredCount });
          }}
        />
        {send}
      </div>
    </Form>
  );
}
