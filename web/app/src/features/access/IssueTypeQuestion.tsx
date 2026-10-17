import { t } from "@bugaboo/messages";
import { SelectField } from "@bugaboo/ui";

import { issueTypeLinesOf, useIssueTypes } from "../issue-types/index.ts";

/** The choice that stands for the levels that count for all issue types. */
const ALL_TYPES = "";

export interface IssueTypeQuestionProps {
  /** The issue type picked (an ID); none for all types. */
  readonly value: string | undefined;
  readonly onChange: (issueType: string | undefined) => void;
}

/**
 * *Which issue type?* (SEC-03, M3-F44): asked only when the levels of the person differ by issue
 * type. The answer below then holds the levels for the type that is picked.
 */
export function IssueTypeQuestion({ value, onChange }: IssueTypeQuestionProps) {
  const types = issueTypeLinesOf(useIssueTypes().data?.pages);
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
