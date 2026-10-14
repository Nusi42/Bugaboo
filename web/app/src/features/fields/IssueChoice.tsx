import { dataOf } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Alert, alertTone, Form, SubmitButton, TextField } from "@bugaboo/ui";
import { useState } from "react";

import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import { ChoiceDialog } from "./ChoiceDialog.tsx";
import type { OtherChoiceProps } from "./OtherChoices.tsx";

/** *Issue*: the readable ID of an issue (like WEB-12), looked up before it is chosen. */
export function IssueChoice({ field, onCommit }: OtherChoiceProps) {
  const [key, setKey] = useState("");
  const [problem, setProblem] = useState<string | undefined>();
  const find = async (done: () => void) => {
    try {
      const found = dataOf(
        await api.client.GET("/api/v1/issues/{issue}", {
          params: { path: { issue: key.trim() } },
        }),
      );
      const id = found.issue.id;
      if (id === undefined || id === null) {
        setProblem(t("fields.pick-no-issue"));
        return;
      }
      setProblem(undefined);
      onCommit({ id, name: key.trim() });
      done();
    } catch (error) {
      setProblem(messageOf(error));
    }
  };
  return (
    <ChoiceDialog field={field}>
      {(done) => (
        <Form
          onSubmit={() => {
            void find(done);
          }}
        >
          <TextField
            label={t("fields.pick-issue-key")}
            requiredMessage={t("validation.required")}
            value={key}
            onChange={setKey}
          />
          {problem === undefined ? null : <Alert tone={alertTone.danger}>{problem}</Alert>}
          <SubmitButton isPending={false}>{t("fields.pick-issue-find")}</SubmitButton>
        </Form>
      )}
    </ChoiceDialog>
  );
}
