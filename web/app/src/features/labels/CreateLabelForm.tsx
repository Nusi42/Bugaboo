import type { Schema } from "@bugaboo/api-client";
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
  type ChoiceColorName,
} from "@bugaboo/ui";
import { useState } from "react";

import { api } from "../../shared/api.ts";
import { ColorChoice } from "../../shared/ColorChoice.tsx";
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

/** What *Create the label* sends: the name, and the color and the label group where they are chosen. */
function bodyOf(
  name: string,
  color: ChoiceColorName | undefined,
  group: string,
): Schema<"NewLabelBody"> {
  return {
    name: name.trim(),
    color: color ?? null,
    label_group: group === NO_GROUP ? null : group,
  };
}

/** *Create a label*: a name, a color and a group; without a color chosen it gets the next in turn, and it can be changed after. */
export function CreateLabelForm({ groups, inGroupsOnly = false }: CreateLabelFormProps) {
  const choices = inGroupsOnly ? managedGroupChoices(groups) : groupChoices(groups);
  const [name, setName] = useState("");
  const [color, setColor] = useState<ChoiceColorName>();
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
      setColor(undefined);
    },
  });
  return (
    <Form
      onSubmit={() => {
        setMade(null);
        create.mutate({ body: bodyOf(name, color, group) });
      }}
    >
      <TextField
        label={t("labels.field-label-name")}
        requiredMessage={t("error.field.required")}
        value={name}
        onChange={setName}
      />
      <ColorChoice legend={t("labels.field-new-color")} value={color} onChange={setColor} />
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
