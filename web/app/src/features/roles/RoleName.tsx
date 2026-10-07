import { t } from "@bugaboo/messages";
import { Alert, Button, Form, TextField } from "@bugaboo/ui";
import { useState } from "react";

import { refreshAccess } from "../../shared/access-cache.ts";
import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import styles from "./Roles.module.css";

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
      {rename.isError ? <Alert danger>{messageOf(rename.error)}</Alert> : null}
      {rename.isSuccess ? <Alert success>{t("roles.saved")}</Alert> : null}
      <div className={styles.row}>
        <Button primary submit isPending={rename.isPending}>
          {t("common.save")}
        </Button>
      </div>
    </Form>
  );
}
