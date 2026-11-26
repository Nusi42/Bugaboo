import { isDateField } from "../../shared/date-fields.ts";
import { OwnFieldInput } from "../fields/index.ts";
import { NewIssueAssignedField } from "./NewIssueAssignedField.tsx";
import { NewIssueDayField } from "./NewIssueDayField.tsx";
import { DescriptionField, EstimateField, PriorityField } from "./NewIssueMoreFields.tsx";
import { NewIssueReleaseField } from "./NewIssueReleaseField.tsx";
import { releaseField } from "./release-fields.ts";
import { withMistake, type NewIssueValues } from "./new-issue-form.ts";

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
      return <PriorityField values={values} onChange={onChange} />;
    case "assigned-to":
      return <NewIssueAssignedField values={values} project={project} onChange={onChange} />;
    case releaseField.found:
    case releaseField.planned:
      return (
        <NewIssueReleaseField word={field} values={values} project={project} onChange={onChange} />
      );
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
            const mended = withMistake(values, field, false);
            onChange({ ...mended, fields: { ...values.fields, [field]: value } });
          }}
          onMistake={() => {
            onChange(withMistake(values, field, true));
          }}
        />
      );
  }
}
