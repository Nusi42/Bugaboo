import { t } from "@bugaboo/messages";
import { SelectField, TextField } from "@bugaboo/ui";

import { ColorChoice } from "../../shared/ColorChoice.tsx";
import { groupChoices, type Label, type LabelGroup } from "./labels.ts";

export interface LabelFieldsProps {
  readonly label: Label;
  readonly groups: readonly LabelGroup[];
  readonly name: string;
  readonly onName: (name: string) => void;
  readonly color: Label["color"];
  readonly onColor: (color: Label["color"]) => void;
  readonly group: string;
  readonly onGroup: (group: string) => void;
}

/** The fields of a label: name, color and group. */
export function LabelFields(props: LabelFieldsProps) {
  return (
    <>
      <TextField
        label={t("labels.field-name")}
        requiredMessage={t("validation.required")}
        value={props.name}
        onChange={props.onName}
      />
      <ColorChoice
        legend={t("labels.field-color", { name: props.label.name })}
        value={props.color}
        onChange={props.onColor}
      />
      <SelectField
        label={t("labels.field-label-group")}
        choices={groupChoices(props.groups)}
        value={props.group}
        onChange={props.onGroup}
      />
    </>
  );
}
