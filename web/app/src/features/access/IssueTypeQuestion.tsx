import { t } from "@bugaboo/messages";
import { SelectField } from "@bugaboo/ui";

/** The choice that stands for the levels that count for all issue types. */
const ALL_TYPES = "";

export interface IssueTypeQuestionProps {
  /** The types the answer judged: the question lists exactly these, and no rule of its own. */
  readonly types: readonly { readonly id: string; readonly name: string }[];
  /** The issue type picked (an ID); none for all types. */
  readonly value: string | undefined;
  readonly onChange: (issueType: string | undefined) => void;
}

/**
 * *Which issue type?* (SEC-03, M3-F44): asked only when the levels of the person differ by issue
 * type. The answer below then holds the levels for the type that is picked.
 */
export function IssueTypeQuestion({ types, value, onChange }: IssueTypeQuestionProps) {
  return (
    <SelectField
      label={t("check-access.issue-type-question")}
      choices={[
        { value: ALL_TYPES, label: t("check-access.issue-type-all") },
        ...types.map((issueType) => ({ value: issueType.id, label: issueType.name })),
      ]}
      value={value ?? ALL_TYPES}
      onChange={(next) => {
        onChange(next === ALL_TYPES ? undefined : next);
      }}
    />
  );
}
