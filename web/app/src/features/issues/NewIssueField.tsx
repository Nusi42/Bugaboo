import { t } from "@bugaboo/messages";
import { Button, buttonLook, MarkdownEditor, SelectField, partOf, part } from "@bugaboo/ui";
import { useState } from "react";

import { useImageHosts } from "../../shared/image-hosts.ts";
import { markdownLabels } from "../../shared/markdown-labels.ts";
import { OwnFieldInput } from "../fields/index.ts";
import { AssigneePicker } from "./AssigneePicker.tsx";
import { priorityChoices } from "./issue-choices.ts";
import type { NewIssueValues } from "./new-issue-form.ts";

export interface NewIssueFieldProps {
  /** The key of the field: the word of a built-in field, or the ID of an own field. */
  readonly field: string;
  readonly values: NewIssueValues;
  /** The project it is made in: its short name. */
  readonly project: string;
  readonly onChange: (values: NewIssueValues) => void;
}

/** *Assigned to* on the form: the people chosen so far, and the button that opens the picker. */
function AssignedField({ values, project, onChange }: Omit<NewIssueFieldProps, "field">) {
  const [picking, setPicking] = useState(false);
  const name = t("issue-field.assigned-to");
  return (
    <div>
      <p {...partOf(part.fieldLabel)}>{name}</p>
      <span {...partOf(part.row)}>
        {values.assignedTo.map((person) => (
          <span key={person.id}>{person.name}</span>
        ))}
      </span>
      <Button
        look={buttonLook.quiet}
        aria-label={
          values.assignedTo.length === 0
            ? t("issues.add-field", { field: name })
            : t("issues.change-field", { field: name })
        }
        onPress={() => {
          setPicking(true);
        }}
      >
        {values.assignedTo.length === 0
          ? t("issues.add-field", { field: name })
          : t("issues.change")}
      </Button>
      <AssigneePicker
        isOpen={picking}
        onClose={() => {
          setPicking(false);
        }}
        project={project}
        issueType={values.issueType === "" ? undefined : values.issueType}
        assigned={values.assignedTo}
        onChange={(assignedTo) => {
          onChange({ ...values, assignedTo });
        }}
      />
    </div>
  );
}

/** One field of the form: a built-in field by its word, an own field by its ID. */
export function NewIssueField(props: NewIssueFieldProps) {
  const { field, values, project, onChange } = props;
  const hosts = useImageHosts();
  switch (field) {
    case "description":
      return (
        <MarkdownEditor
          label={t("issue-field.description")}
          labels={markdownLabels()}
          value={values.description}
          onChange={(description) => {
            onChange({ ...values, description });
          }}
          imageHosts={hosts}
        />
      );
    case "priority":
      return (
        <SelectField
          label={t("issue-field.priority")}
          choices={priorityChoices}
          value={values.priority}
          onChange={(priority) => {
            onChange({ ...values, priority });
          }}
        />
      );
    case "assigned-to":
      return <AssignedField values={values} project={project} onChange={onChange} />;
    default:
      return (
        <OwnFieldInput
          field={field}
          project={project}
          issueType={values.issueType === "" ? undefined : values.issueType}
          value={values.fields[field]}
          onChange={(value) => {
            onChange({
              ...values,
              fields: { ...values.fields, [field]: value },
            });
          }}
        />
      );
  }
}
