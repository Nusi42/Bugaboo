import { t } from "@bugaboo/messages";
import { Alert, alertTone, Form, SubmitButton, TextField, partOf, part } from "@bugaboo/ui";
import { useState } from "react";

import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import { refreshRelationKinds } from "../../shared/query-keys.ts";

/** *Create a kind of relation*: the words for both ends; the kind goes to the end of the list. */
export function CreateRelationKindForm() {
  const [forward, setForward] = useState("");
  const [backward, setBackward] = useState("");
  const [made, setMade] = useState<string | null>(null);
  const create = api.query.useMutation("post", "/api/v1/relation-kinds", {
    onSuccess: (kind) => {
      refreshRelationKinds();
      setForward("");
      setBackward("");
      setMade(kind.forward ?? forward);
    },
  });
  return (
    <Form
      onSubmit={() => {
        setMade(null);
        create.mutate({ body: { forward: forward.trim(), backward: backward.trim() } });
      }}
    >
      <TextField
        label={t("relation-kinds.field-forward")}
        requiredMessage={t("error.field.required")}
        value={forward}
        onChange={setForward}
      />
      <TextField
        label={t("relation-kinds.field-backward")}
        requiredMessage={t("error.field.required")}
        value={backward}
        onChange={setBackward}
      />
      {create.isError ? <Alert tone={alertTone.danger}>{messageOf(create.error)}</Alert> : null}
      {made === null ? null : (
        <Alert tone={alertTone.success}>{t("relation-kinds.made", { name: made })}</Alert>
      )}
      <div {...partOf(part.row)}>
        <SubmitButton isPending={create.isPending}>{t("relation-kinds.create")}</SubmitButton>
      </div>
    </Form>
  );
}
