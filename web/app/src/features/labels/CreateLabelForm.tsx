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
import {
  NO_GROUP,
  groupChoices,
  managedGroupChoices,
  refreshLabels,
  type LabelGroup,
} from "./labels.ts";

export interface CreateLabelFormProps {
  readonly groups: readonly LabelGroup[];
  /** The form of a project: a label goes into a group the person looks after, never into none. */
  readonly inGroupsOnly?: boolean;
}

/** *Create a label*: a name and a group; its color is the next in turn, and it can be changed after. */
export function CreateLabelForm({ groups, inGroupsOnly = false }: CreateLabelFormProps) {
  const choices = inGroupsOnly ? managedGroupChoices(groups) : groupChoices(groups);
  const [name, setName] = useState("");
  const [picked, setGroup] = useState(NO_GROUP);
  const group = choices.some((choice) => choice.value === picked)
    ? picked
    : (choices[0]?.value ?? NO_GROUP);
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
        choices={choices}
        value={group}
        onChange={setGroup}
      />
      {create.isError ? <Alert tone={alertTone.danger}>{messageOf(create.error)}</Alert> : null}
      {made === null ? null : (
        <Alert tone={alertTone.success}>{t("labels.label-created", { name: made })}</Alert>
      )}
      <div {...partOf(part.row)}>
        <SubmitButton isPending={create.isPending}>{t("labels.create-label")}</SubmitButton>
      </div>
    </Form>
  );
}
