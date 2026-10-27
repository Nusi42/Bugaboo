import { dataOf } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Alert, alertTone } from "@bugaboo/ui";
import { useState } from "react";

import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import { IssuePicker } from "../search/index.ts";
import { ChoiceDialog } from "./ChoiceDialog.tsx";
import type { OtherChoiceProps } from "./OtherChoices.tsx";

/**
 * *Issue*: an issue found by its readable ID (like WEB-12) or by words of its title (M3-F62),
 * looked up before it is chosen.
 */
export function IssueChoice({ field, onCommit }: OtherChoiceProps) {
  const [problem, setProblem] = useState<string | undefined>();
  const find = async (key: string, done: () => void) => {
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
      onCommit({ id, name: found.issue.readable_id ?? key.trim() });
      done();
    } catch (error) {
      setProblem(messageOf(error));
    }
  };
  return (
    <ChoiceDialog field={field}>
      {(done) => (
        <>
          <IssuePicker
            onPick={(key) => {
              void find(key, done);
            }}
          />
          {problem === undefined ? null : <Alert tone={alertTone.danger}>{problem}</Alert>}
        </>
      )}
    </ChoiceDialog>
  );
}
