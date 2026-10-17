import { t } from "@bugaboo/messages";
import { Alert, alertTone, Form, SubmitButton, Switch, TextField, partOf, part } from "@bugaboo/ui";
import { useState } from "react";

import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import { refreshLabels } from "./labels.ts";

export interface CreateLabelGroupFormProps {
  /** The project the group is created in (a short name); the whole organization when left out. */
  readonly project?: string;
}

/** *Create a group of labels*: a name, and whether an issue gets at most one label of it. */
export function CreateLabelGroupForm({ project }: CreateLabelGroupFormProps) {
  const [name, setName] = useState("");
  const [pickOne, setPickOne] = useState(false);
  const [made, setMade] = useState<string | null>(null);
  const create = api.query.useMutation("post", "/api/v1/label-groups", {
    onSuccess: (group) => {
      refreshLabels();
      setMade(group.name);
      setName("");
      setPickOne(false);
    },
  });
  return (
    <Form
      onSubmit={() => {
        setMade(null);
        create.mutate({
          body: {
            name: name.trim(),
            pick_one: pickOne,
            ...(project === undefined ? {} : { project }),
          },
        });
      }}
    >
      <TextField
        label={t("labels.field-group-name")}
        requiredMessage={t("error.field.required")}
        value={name}
        onChange={setName}
      />
      <Switch label={t("labels.field-pick-one")} isSelected={pickOne} onChange={setPickOne} />
      {create.isError ? <Alert tone={alertTone.danger}>{messageOf(create.error)}</Alert> : null}
      {made === null ? null : (
        <Alert tone={alertTone.success}>{t("labels.group-created", { name: made })}</Alert>
      )}
      <div {...partOf(part.row)}>
        <SubmitButton isPending={create.isPending}>{t("labels.create-label-group")}</SubmitButton>
      </div>
    </Form>
  );
}
