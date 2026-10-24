import { t } from "@bugaboo/messages";
import { Alert, alertTone, Form, SubmitButton, TextField, partOf, part } from "@bugaboo/ui";
import { useState } from "react";

import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import { refreshAccess } from "../../shared/query-keys.ts";
import type { GroupLine } from "./groups.ts";

/** *Rename*: the name of the group, saved with the button. */
export function RenameGroup({ group }: { readonly group: GroupLine }) {
  const [name, setName] = useState(group.name);
  const rename = api.query.useMutation("patch", "/api/v1/groups/{group}", {
    onSuccess: refreshAccess,
  });
  return (
    <Form
      onSubmit={() => {
        rename.mutate({
          params: { path: { group: group.id } },
          body: { name: name.trim(), version: group.version },
        });
      }}
    >
      <TextField
        label={t("groups.field-name")}
        requiredMessage={t("error.field.required")}
        value={name}
        onChange={setName}
      />
      {rename.isError ? <Alert tone={alertTone.danger}>{messageOf(rename.error)}</Alert> : null}
      {rename.isSuccess ? <Alert tone={alertTone.success}>{t("save.saved")}</Alert> : null}
      <div {...partOf(part.row)}>
        <SubmitButton isPending={rename.isPending}>{t("common.save")}</SubmitButton>
      </div>
    </Form>
  );
}
