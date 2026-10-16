import { t } from "@bugaboo/messages";
import { Alert, alertTone, Form, SubmitButton, TextField, partOf, part } from "@bugaboo/ui";
import { useState } from "react";

import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import { refreshAccess } from "../../shared/query-keys.ts";

/** *Create a group*: a name; the group starts empty. */
export function CreateGroupForm() {
  const [name, setName] = useState("");
  const [made, setMade] = useState<string | null>(null);
  const create = api.query.useMutation("post", "/api/v1/groups", {
    onSuccess: (group) => {
      refreshAccess();
      setName("");
      setMade(group.name);
    },
  });
  return (
    <Form
      onSubmit={() => {
        setMade(null);
        create.mutate({ body: { name: name.trim() } });
      }}
    >
      <TextField
        label={t("groups.field-name")}
        requiredMessage={t("error.field.required")}
        value={name}
        onChange={setName}
      />
      {create.isError ? <Alert tone={alertTone.danger}>{messageOf(create.error)}</Alert> : null}
      {made === null ? null : (
        <Alert tone={alertTone.success}>{t("groups.made", { name: made })}</Alert>
      )}
      <div {...partOf(part.row)}>
        <SubmitButton isPending={create.isPending}>{t("groups.create")}</SubmitButton>
      </div>
    </Form>
  );
}
