import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Alert, Button, Form, TextField } from "@bugaboo/ui";
import { useState } from "react";
import { useNavigate } from "@tanstack/react-router";

import { refreshAccess } from "../../shared/query-keys.ts";
import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import { roleAddress } from "../../shared/paths.ts";
import styles from "./Roles.module.css";

export interface CopyRoleProps {
  readonly id: string;
  readonly name: string;
}

/** *Copy this role*: a new role with the same permissions, which opens on its own page. */
export function CopyRole({ id, name }: CopyRoleProps) {
  const [typed, setTyped] = useState(t("roles.copy-of", { name }));
  const navigate = useNavigate();
  const copy = api.query.useMutation("post", "/api/v1/roles/{role_id}/copy", {
    onSuccess: (made: Schema<"RoleBody">) => {
      refreshAccess();
      void navigate({ to: roleAddress(made.id) });
    },
  });
  return (
    <Form
      onSubmit={() => {
        copy.mutate({ params: { path: { role_id: id } }, body: { name: typed.trim() } });
      }}
    >
      <p className={styles.muted}>{t("roles.copy-help")}</p>
      <TextField
        label={t("roles.copy-field")}
        requiredMessage={t("validation.required")}
        value={typed}
        onChange={setTyped}
      />
      {copy.isError ? <Alert danger>{messageOf(copy.error)}</Alert> : null}
      <div className={styles.row}>
        <Button submit isPending={copy.isPending}>
          {t("roles.copy")}
        </Button>
      </div>
    </Form>
  );
}
