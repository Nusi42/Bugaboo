import { t } from "@bugaboo/messages";
import { MarkdownEditor, SelectField, partOf, part } from "@bugaboo/ui";
import { useState } from "react";

import { useImageHosts } from "../../shared/image-hosts.ts";
import { markdownLabels } from "../../shared/markdown-labels.ts";
import { DateBox, LengthBox, OwnFieldInput } from "../fields/index.ts";
import { AssignedPicker } from "./AssignedPicker.tsx";
import { ChangeAssigned } from "./AssignedRow.tsx";
import { priorityChoices } from "./issue-choices.ts";
import type { NewIssueValues } from "./new-issue-form.ts";

export interface NewIssueFieldProps {
  /** The key of the field: the word of a built-in field, or the ID of an own field. */
  readonly field: string;
  readonly values: NewIssueValues;
  /** The project it is made in: its short name. */
  readonly project: string;
  /** What the server said was wrong with this field. */
  readonly problem?: string | undefined;
  readonly onChange: (values: NewIssueValues) => void;
}

/** *Assigned to* on the form: the people chosen so far, and the button that opens the picker. */
function AssignedField({
  values,
  project,
  onChange,
}: Omit<NewIssueFieldProps, "field" | "problem">) {
  const [picking, setPicking] = useState(false);
  const name = t("issue-field.assigned-to");
  return (
    <div>
      <p {...partOf(part.fieldLabel)}>{name}</p>
      <div {...partOf(part.startColumn)}>
        {values.assignedTo.map((person) => (
          <span key={person.id}>{person.name}</span>
        ))}
        <ChangeAssigned
          name={name}
          empty={values.assignedTo.length === 0}
          onPress={() => {
            setPicking(true);
          }}
        />
      </div>
      <AssignedPicker
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

/** *Estimated time* on the form: a length the person types; a text that is none holds the form back. */
function EstimateField({
  values,
  problem,
  onChange,
}: Omit<NewIssueFieldProps, "field" | "project">) {
  return (
    <LengthBox
      name={t("issue-field.estimated-time")}
      minutes={values.estimatedTime ?? undefined}
      inRow={false}
      problem={problem}
      onCommit={(estimatedTime) => {
        onChange({ ...values, estimatedTime, estimateWrong: false });
      }}
      onMistake={() => {
        onChange({ ...values, estimateWrong: true });
      }}
    />
  );
}

/** *Start date* or *Due date* on the form: a day the person types; the server judges the two together. */
function DayField({ field, values, problem, onChange }: Omit<NewIssueFieldProps, "project">) {
  const start = field === "start-date";
  return (
    <DateBox
      name={t(start ? "issue-field.start-date" : "issue-field.due-date")}
      day={(start ? values.startDate : values.dueDate) || undefined}
      inRow={false}
      problem={problem}
      onCommit={(day) => {
        onChange(start ? { ...values, startDate: day ?? "" } : { ...values, dueDate: day ?? "" });
      }}
    />
  );
}

/** *Description* on the form: the editor of text, with the hosts its images may come from. */
function DescriptionField({ values, onChange }: Pick<NewIssueFieldProps, "values" | "onChange">) {
  const hosts = useImageHosts();
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
}

/** One field of the form: a built-in field by its word, an own field by its ID. */
export function NewIssueField(props: NewIssueFieldProps) {
  const { field, values, project, problem, onChange } = props;
  switch (field) {
    case "description":
      return <DescriptionField values={values} onChange={onChange} />;
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
    case "start-date":
    case "due-date":
      return <DayField {...props} />;
    case "estimated-time":
      return <EstimateField values={values} problem={problem} onChange={onChange} />;
    default:
      return (
        <OwnFieldInput
          field={field}
          project={project}
          issueType={values.issueType === "" ? undefined : values.issueType}
          value={values.fields[field]}
          problem={problem}
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
