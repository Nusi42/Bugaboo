import { t } from "@bugaboo/messages";
import { Alert, Button, Form, TextField } from "@bugaboo/ui";
import { useState } from "react";

import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import { refreshAccess } from "../../shared/access-cache.ts";
import styles from "./People.module.css";

/** *Make a group*: a name; the group starts empty. */
export function MakeGroupForm() {
  const [name, setName] = useState("");
  const [made, setMade] = useState<string | null>(null);
  const make = api.query.useMutation("post", "/api/v1/groups", {
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
        make.mutate({ body: { name: name.trim() } });
      }}
    >
      <TextField
        label={t("groups.field-name")}
        requiredMessage={t("validation.required")}
        value={name}
        onChange={setName}
      />
      {make.isError ? <Alert danger>{messageOf(make.error)}</Alert> : null}
      {made === null ? null : <Alert success>{t("groups.made", { name: made })}</Alert>}
      <div className={styles.row}>
        <Button primary submit isPending={make.isPending}>
          {t("groups.make")}
        </Button>
      </div>
    </Form>
  );
}
