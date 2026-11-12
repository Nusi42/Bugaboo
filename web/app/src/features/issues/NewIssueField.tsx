import { t } from "@bugaboo/messages";
import { MarkdownEditor, SelectField } from "@bugaboo/ui";

import { useImageHosts } from "../../shared/image-hosts.ts";
import { markdownLabels } from "../../shared/markdown-labels.ts";
import { isDateField } from "../../shared/date-fields.ts";
import { LengthBox, OwnFieldInput } from "../fields/index.ts";
import { priorityChoices } from "./issue-choices.ts";
import { NewIssueAssignedField } from "./NewIssueAssignedField.tsx";
import { NewIssueDayField } from "./NewIssueDayField.tsx";
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
  if (isDateField(field)) {
    return <NewIssueDayField field={field} values={values} problem={problem} onChange={onChange} />;
  }
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
      return <NewIssueAssignedField values={values} project={project} onChange={onChange} />;
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
