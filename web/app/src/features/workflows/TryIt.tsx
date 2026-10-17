import { t } from "@bugaboo/messages";
import { Alert, alertTone, Button, Card, TextField, partOf, part } from "@bugaboo/ui";
import { useState } from "react";

import { messageOf } from "../../shared/errors.ts";
import { fieldNameOf } from "../../shared/field-names.ts";
import { listText } from "../../shared/list-text.ts";
import { PersonPicker } from "../access/index.ts";
import { useTryIt, type Tried, type WorkflowEditor } from "./workflows.ts";

type Verdict = Tried["changes"][number];

/** The fields of the editor by the names people read. */
function namesOf(fields: WorkflowEditor["fields"], keys: readonly string[]): string {
  return listText(
    keys.map((key) => {
      const found = fields.find((each) => each.field === key);
      return fieldNameOf({ code: key, name: found?.name });
    }),
  );
}

/** What the judge says about one status change, in a sentence. */
function sentenceOf(change: Verdict, fields: WorkflowEditor["fields"]): string {
  if (change.verdict === "allowed") {
    return t("workflows.try-allowed");
  }
  if (change.verdict === "needs") {
    return t("workflows.try-needs", { fields: namesOf(fields, change.fields) });
  }
  const reason = t(`error.workflow.${change.why}`);
  return change.field === undefined || change.field === null
    ? reason
    : t("workflows.try-on-field", { reason, field: namesOf(fields, [change.field]) });
}

/** The status changes from the status the issue is in, each with the answer for the person. */
function Result({
  tried,
  issue,
  fields,
}: {
  readonly tried: Tried;
  readonly issue: string;
  readonly fields: WorkflowEditor["fields"];
}) {
  return (
    <div {...partOf(part.stack)}>
      <p {...partOf(part.text)}>
        {t("workflows.try-in", { key: issue, status: tried.status.name })}
      </p>
      {tried.changes.length === 0 ? (
        <p {...partOf(part.muted)}>{t("workflows.try-none")}</p>
      ) : (
        <ul {...partOf(part.workflowTried)}>
          {tried.changes.map((change) => (
            <li key={change.to.key} {...partOf(part.workflowTry)}>
              <strong>{change.button}</strong>
              <span {...partOf(part.muted)}>{change.to.name}</span>
              <span>{sentenceOf(change, fields)}</span>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

/**
 * *Try it* (WF-02, UX §5.4): pick an issue and a person and see which status changes they would
 * get, and why not the others. The server answers with the same judge the issue page uses.
 */
export function TryIt({ fields }: { readonly fields: WorkflowEditor["fields"] }) {
  const [issue, setIssue] = useState("");
  const [person, setPerson] = useState<string | undefined>();
  const tried = useTryIt();
  return (
    <Card heading={t("workflows.try-title")}>
      <p {...partOf(part.muted)}>{t("workflows.try-text")}</p>
      <TextField label={t("common.issue-id")} value={issue} onChange={setIssue} />
      <PersonPicker project={undefined} value={person} onChange={setPerson} />
      <div {...partOf(part.row)}>
        <Button
          isDisabled={issue.trim() === "" || person === undefined}
          isPending={tried.isPending}
          onPress={() => {
            if (person !== undefined) {
              tried.mutate({ issue: issue.trim(), person });
            }
          }}
        >
          {t("workflows.try-go")}
        </Button>
      </div>
      {tried.isError ? <Alert tone={alertTone.danger}>{messageOf(tried.error)}</Alert> : null}
      {tried.data === undefined ? null : (
        <Result tried={tried.data} issue={issue.trim()} fields={fields} />
      )}
    </Card>
  );
}
