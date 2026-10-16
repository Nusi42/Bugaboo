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
import { useState } from "react";

import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import { NO_GROUP, groupChoices, refreshLabels, type LabelGroup } from "./labels.ts";

/** *Create a label*: a name and a group; its color is the next in turn, and it can be changed after. */
export function CreateLabelForm({ groups }: { readonly groups: readonly LabelGroup[] }) {
  const [name, setName] = useState("");
  const [group, setGroup] = useState(NO_GROUP);
  const [made, setMade] = useState<string | null>(null);
  const create = api.query.useMutation("post", "/api/v1/labels", {
    onSuccess: (label) => {
      refreshLabels();
      setMade(label.name);
      setName("");
    },
  });
  return (
    <Form
      onSubmit={() => {
        setMade(null);
        create.mutate({ body: { name: name.trim(), ...(group === NO_GROUP ? {} : { group }) } });
      }}
    >
      <TextField
        label={t("labels.field-label-name")}
        requiredMessage={t("error.field.required")}
        value={name}
        onChange={setName}
      />
      <SelectField
        label={t("labels.field-label-group")}
        choices={groupChoices(groups)}
        value={group}
        onChange={setGroup}
      />
      {create.isError ? <Alert tone={alertTone.danger}>{messageOf(create.error)}</Alert> : null}
      {made === null ? null : (
        <Alert tone={alertTone.success}>{t("labels.label-made", { name: made })}</Alert>
      )}
      <div {...partOf(part.row)}>
        <SubmitButton isPending={create.isPending}>{t("labels.create-label")}</SubmitButton>
      </div>
    </Form>
  );
}
