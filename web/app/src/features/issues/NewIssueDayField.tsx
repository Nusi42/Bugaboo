import { t } from "@bugaboo/messages";

import { dateField, dateFieldName, type DateField } from "../../shared/date-fields.ts";
import { DateBox } from "../fields/index.ts";
import type { NewIssueValues } from "./new-issue-form.ts";

export interface NewIssueDayFieldProps {
  readonly field: DateField;
  readonly values: NewIssueValues;
  /** What the server said was wrong with this field. */
  readonly problem?: string | undefined;
  readonly onChange: (values: NewIssueValues) => void;
}

/** *Start date* or *Due date* on the form: a day the person types; the server judges the two together. */
export function NewIssueDayField({ field, values, problem, onChange }: NewIssueDayFieldProps) {
  const start = field === dateField.start;
  return (
    <DateBox
      name={t(dateFieldName[field])}
      day={(start ? values.startDate : values.dueDate) || undefined}
      inRow={false}
      problem={problem}
      onCommit={(day) => {
        onChange(start ? { ...values, startDate: day ?? "" } : { ...values, dueDate: day ?? "" });
      }}
    />
  );
}
