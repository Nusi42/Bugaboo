import { t } from "@bugaboo/messages";
import { Alert, alertTone, Form, SubmitButton, TextField, partOf, part } from "@bugaboo/ui";
import { useState } from "react";

import { refreshAccess } from "../../shared/query-keys.ts";
import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";

export interface RoleNameProps {
  readonly id: string;
  readonly name: string;
}

/** *Rename*: the name of the role, saved with the button. A starting role can be renamed too. */
export function RoleName({ id, name }: RoleNameProps) {
  const [typed, setTyped] = useState(name);
  const rename = api.query.useMutation("patch", "/api/v1/roles/{role_id}", {
    onSuccess: refreshAccess,
  });
  return (
    <Form
      onSubmit={() => {
        rename.mutate({ params: { path: { role_id: id } }, body: { name: typed.trim() } });
      }}
    >
      <TextField
        label={t("roles.field-name")}
        requiredMessage={t("validation.required")}
        value={typed}
        onChange={setTyped}
      />
      {rename.isError ? <Alert tone={alertTone.danger}>{messageOf(rename.error)}</Alert> : null}
      {rename.isSuccess ? <Alert tone={alertTone.success}>{t("roles.saved")}</Alert> : null}
      <div {...partOf(part.row)}>
        <SubmitButton isPending={rename.isPending}>{t("common.save")}</SubmitButton>
      </div>
    </Form>
  );
}
