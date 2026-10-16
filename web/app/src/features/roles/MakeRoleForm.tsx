import { t } from "@bugaboo/messages";
import {
  Alert,
  alertTone,
  Form,
  SelectField,
  SubmitButton,
  TextField,
  partOf,
  part,
} from "@bugaboo/ui";
import { useNavigate } from "@tanstack/react-router";
import { useState } from "react";

import { refreshAccess } from "../../shared/query-keys.ts";
import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import { roleAddress } from "../../shared/paths.ts";
import { roleLinesOf, useRoles } from "./roles.ts";

/** The word that stands for *An empty role* among the roles to start from. */
const EMPTY = "";

/**
 * *Make a role* (SEC-08): a name, and where to start: an empty role, or a copy of a role that is
 * close to what is needed. The new role opens on its own page, to choose what it allows.
 */
export function MakeRoleForm() {
  const [name, setName] = useState("");
  const [from, setFrom] = useState(EMPTY);
  const navigate = useNavigate();
  const roles = useRoles();
  const opened = (made: { readonly id: string }): void => {
    refreshAccess();
    void navigate({ to: roleAddress(made.id) });
  };
  const empty = api.query.useMutation("post", "/api/v1/roles", { onSuccess: opened });
  const copy = api.query.useMutation("post", "/api/v1/roles/{role_id}/copy", {
    onSuccess: opened,
  });
  const problem = empty.error ?? copy.error;
  const start = [
    { value: EMPTY, label: t("roles.start-empty") },
    ...roleLinesOf(roles.data?.pages).map((role) => ({
      value: role.id,
      label: t("roles.start-copy", { name: role.name }),
    })),
  ];
  return (
    <Form
      onSubmit={() => {
        const typed = name.trim();
        if (from === EMPTY) {
          empty.mutate({ body: { name: typed, permissions: [], two_step: false } });
        } else {
          copy.mutate({ params: { path: { role_id: from } }, body: { name: typed } });
        }
      }}
    >
      <TextField
        label={t("roles.field-name")}
        requiredMessage={t("error.field.required")}
        value={name}
        onChange={setName}
      />
      <SelectField label={t("roles.start-from")} choices={start} value={from} onChange={setFrom} />
      {problem === null ? null : <Alert tone={alertTone.danger}>{messageOf(problem)}</Alert>}
      <div {...partOf(part.row)}>
        <SubmitButton isPending={empty.isPending || copy.isPending}>
          {t("roles.create")}
        </SubmitButton>
      </div>
    </Form>
  );
}
