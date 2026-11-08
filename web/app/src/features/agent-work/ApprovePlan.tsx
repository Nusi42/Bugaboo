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
 * *Approve plan* under the plan (UX §4.17): one note field and one filled button. Without a note the
 * button approves the plan and lets the attempt start; with a note it is named for what it does,
 * answers the plan and the agent makes another. Both are sent against the version of the work the
 * person saw.
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
  // One note field and one button: the button approves the plan, and with a note it answers the plan instead.
  return (
    <div {...partOf(part.agentWorkQuestion)}>
      <TextField
        label={t("agent-work.plan-note")}
        description={t("agent-work.plan-note-help")}
        kind={fieldKind.multiline}
        value={note}
        onChange={setNote}
      />
      {send.isError ? <Alert tone={alertTone.danger}>{messageOf(send.error)}</Alert> : null}
      <div {...partOf(part.agentWorkActions)}>
        <Button
          look={buttonLook.primary}
          isPending={send.isPending}
          onPress={() => {
            ask(!nothingWritten);
          }}
        >
          {nothingWritten ? t("agent-work.plan-approve") : t("agent-work.plan-answer")}
        </Button>
      </div>
    </div>
  );
}
