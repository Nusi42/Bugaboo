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
} from "@bugaboo/ui";
import { useState } from "react";

import { messageOf } from "../../shared/errors.ts";
import { limits } from "./assistant.ts";

export interface AskBoxProps {
  /** Sends the question; resolves when the answer is there, and says whether to empty the box. */
  readonly onAsk: (text: string) => void;
  readonly isPending: boolean;
  readonly error: unknown;
  /** The box is emptied when this changes: a question was answered. */
  readonly answeredCount: number;
}

/** The box to ask, with its button; the question is cut at the limit of the server. */
export function AskBox({ onAsk, isPending, error, answeredCount }: AskBoxProps) {
  const [typed, setTyped] = useState({ text: "", after: answeredCount });
  // A question that was answered leaves the box empty; one that failed stays, to be sent again.
  const text = typed.after === answeredCount ? typed.text : "";
  return (
    <Form
      onSubmit={() => {
        if (text.trim() !== "") {
          onAsk(text.trim().slice(0, limits.question));
          setTyped({ text, after: answeredCount });
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
          kind={fieldKind.multiline}
          value={text}
          onChange={(next) => {
            setTyped({ text: next, after: answeredCount });
          }}
        />
        <div {...partOf(part.row)}>
          <SubmitButton isPending={isPending}>{t("assistant.ask")}</SubmitButton>
        </div>
      </div>
    </Form>
  );
}
