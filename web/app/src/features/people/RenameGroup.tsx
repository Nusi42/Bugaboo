import { t } from "@bugaboo/messages";
import { Alert, Button, Form, TextField } from "@bugaboo/ui";
import { useState } from "react";

import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import { refreshAccess } from "../../shared/access-cache.ts";
import type { GroupLine } from "./groups.ts";
import styles from "./People.module.css";

/** *Rename*: the name of the group, saved with the button. */
export function RenameGroup({ group }: { readonly group: GroupLine }) {
  const [name, setName] = useState(group.name);
  const rename = api.query.useMutation("patch", "/api/v1/groups/{group_id}", {
    onSuccess: refreshAccess,
  });
  return (
    <Form
      onSubmit={() => {
        rename.mutate({ params: { path: { group_id: group.id } }, body: { name: name.trim() } });
      }}
    >
      <TextField
        label={t("groups.field-name")}
        requiredMessage={t("validation.required")}
        value={name}
        onChange={setName}
      />
      {rename.isError ? <Alert danger>{messageOf(rename.error)}</Alert> : null}
      {rename.isSuccess ? <Alert success>{t("groups.saved")}</Alert> : null}
      <div className={styles.row}>
        <Button primary submit isPending={rename.isPending}>
          {t("common.save")}
        </Button>
      </div>
    </Form>
  );
}
