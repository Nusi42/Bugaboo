import { dataOf, reasonMax } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import {
  Alert,
  alertTone,
  autoComplete,
  Card,
  fieldKind,
  Form,
  keepsRoom,
  SaveStatus,
  SubmitButton,
  TextField,
  partOf,
  part,
  type FieldKind,
  type SaveState,
} from "@bugaboo/ui";
import { useState } from "react";

import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import { queryClient } from "../../shared/query-client.ts";
import { queryKeys } from "../../shared/query-keys.ts";
import { saveLabels } from "../../shared/save-labels.ts";

interface CorrectFormProps {
  /** What is corrected: the name of the box. */
  readonly label: string;
  readonly kind: FieldKind;
  /** The value the person has now. */
  readonly value: string;
  /** The words of the button. */
  readonly submit: string;
  /** Sends the new value with the reason; settles with the problem, if one. */
  readonly onCorrect: (value: string, reason: string) => Promise<unknown>;
}

/**
 * One correction: the new value and the reason, which the person is told. What came of it stands
 * beside the button in a place that keeps its room, so the button never moves (UX §6.4); a problem
 * stands under the row.
 */
function CorrectForm({ label, kind, value, submit, onCorrect }: CorrectFormProps) {
  const [draft, setDraft] = useState(value);
  const [reason, setReason] = useState("");
  const [state, setState] = useState<SaveState>("idle");
  const [problem, setProblem] = useState<string | null>(null);
  const pending = state === "saving";
  return (
    <Form
      onSubmit={() => {
        if (pending) {
          return;
        }
        setState("saving");
        setProblem(null);
        onCorrect(draft.trim(), reason.trim()).then(
          () => {
            setState("saved");
          },
          (error: unknown) => {
            setState("not-saved");
            setProblem(messageOf(error));
          },
        );
      }}
    >
      <TextField
        label={label}
        kind={kind}
        autoComplete={autoComplete.off}
        requiredMessage={t("error.field.required")}
        value={draft}
        onChange={setDraft}
      />
      <TextField
        label={t("person.reason")}
        description={t("person.reason-help")}
        requiredMessage={t("error.field.required")}
        maxLength={reasonMax}
        value={reason}
        onChange={setReason}
      />
      <div {...partOf(part.row)}>
        <SubmitButton isPending={pending}>{submit}</SubmitButton>
        <SaveStatus state={state} labels={correctedLabels()} variant={keepsRoom} />
      </div>
      {problem === null ? null : <Alert tone={alertTone.danger}>{problem}</Alert>}
    </Form>
  );
}

/** The words of a correction's state: *Corrected. The person is told.* once it is done. */
function correctedLabels() {
  return { ...saveLabels(), saved: t("person.corrected") };
}

/** The person as the page shows them again after a correction. */
function refresh(id: string) {
  void queryClient.invalidateQueries({ queryKey: queryKeys.people.one(id) });
}

/**
 * The corrections of the name and the e-mail address of a person (PRIV-06, ADR-0044, decision 9),
 * for whoever holds *Manage people*, which the page itself needs. Each asks for a reason.
 */
export function Corrections({
  id,
  name,
  email,
}: {
  readonly id: string;
  readonly name: string;
  readonly email: string;
}) {
  const path = { params: { path: { person: id } } };
  return (
    <Card heading={t("person.corrections")}>
      <CorrectForm
        label={t("person.new-name")}
        kind={fieldKind.text}
        value={name}
        submit={t("person.correct-name")}
        onCorrect={async (value, reason) => {
          dataOf(
            await api.client.PUT("/api/v1/people/{person}/name", {
              ...path,
              body: { name: value, reason },
            }),
          );
          refresh(id);
        }}
      />
      <CorrectForm
        label={t("person.new-e-mail")}
        kind={fieldKind.email}
        value={email}
        submit={t("person.correct-e-mail")}
        onCorrect={async (value, reason) => {
          dataOf(
            await api.client.PUT("/api/v1/people/{person}/e-mail", {
              ...path,
              body: { email: value, reason },
            }),
          );
          refresh(id);
        }}
      />
    </Card>
  );
}
