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
import { useState } from "react";

import { messageOf } from "../../shared/errors.ts";
import { limits } from "./assistant.ts";

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
  const tooLong = text.trim().length > limits.question;
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
        <div {...partOf(part.row)}>
          <SubmitButton isPending={isPending} isDisabled={isDisabled || tooLong} wide={phone}>
            {t("assistant.ask")}
          </SubmitButton>
        </div>
      </div>
    </Form>
  );
}
