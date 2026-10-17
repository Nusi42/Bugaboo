import { t } from "@bugaboo/messages";
import { SelectField } from "@bugaboo/ui";

import { issueTypeLinesOf, useIssueTypes } from "../issue-types/index.ts";

/** The choice that stands for the levels that count for all issue types. */
const ALL_TYPES = "";

export interface IssueTypeQuestionProps {
  /** The project the levels are looked at (an ID); none for the whole organization. */
  readonly project: string | undefined;
  /** The issue type picked (an ID); none for all types. */
  readonly value: string | undefined;
  readonly onChange: (issueType: string | undefined) => void;
}

/**
 * *Which issue type?* (SEC-03, M3-F44): asked only when the levels of the person differ by issue
 * type. The answer below then holds the levels for the type that is picked.
 */
export function IssueTypeQuestion({ project, value, onChange }: IssueTypeQuestionProps) {
  // Only the types that are offered in the place and not retired: they are the ones to ask about.
  // A list about a project holds only the types that can reach it; the whole organization's list
  // holds the types created for it.
  const types = issueTypeLinesOf(useIssueTypes(project).data?.pages).filter(
    (issueType) =>
      !issueType.retired &&
      (project === undefined
        ? (issueType.createdIn?.kind ?? "organization") === "organization"
        : issueType.here?.offered === true),
  );
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
