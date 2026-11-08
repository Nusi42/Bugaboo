import { t } from "@bugaboo/messages";
import {
  Alert,
  alertTone,
  Button,
  buttonLook,
  fieldKind,
  partOf,
  part,
  TextField,
} from "@bugaboo/ui";
import { useState } from "react";

import { messageOf } from "../../shared/errors.ts";
import { useApprovePlan, type Work } from "./agent-work.ts";

/**
 * *Approve plan* under the plan (UX §4.17): the one filled button, which lets the attempt start.
 * A note, which is optional, answers the plan instead and the agent makes another. Both are sent
 * against the version of the work the person saw.
 */
export function ApprovePlan({ work }: { readonly work: Work }) {
  const [note, setNote] = useState("");
  const send = useApprovePlan();
  const nothingWritten = note.trim().length === 0;
  const ask = (withNote: boolean): void => {
    send.mutate({
      params: { path: { work: work.id } },
      body: { version: work.version, ...(withNote ? { note } : {}) },
    });
  };
  return (
    <div {...partOf(part.agentWorkQuestion)}>
      <div {...partOf(part.agentWorkActions)}>
        <Button
          look={buttonLook.primary}
          isPending={send.isPending}
          onPress={() => {
            ask(false);
          }}
        >
          {t("agent-work.plan-approve")}
        </Button>
      </div>
      <TextField
        label={t("agent-work.plan-note")}
        kind={fieldKind.multiline}
        value={note}
        onChange={setNote}
      />
      {send.isError ? <Alert tone={alertTone.danger}>{messageOf(send.error)}</Alert> : null}
      <div {...partOf(part.agentWorkActions)}>
        <Button
          isDisabled={nothingWritten}
          onPress={() => {
            ask(true);
          }}
        >
          {t("agent-work.plan-answer")}
        </Button>
      </div>
    </div>
  );
}
