import { t } from "@bugaboo/messages";
import { SelectField } from "@bugaboo/ui";

import { issueTypeLinesOf, useIssueTypes } from "./issue-types.ts";

/** What the list holds for all issue types: the word of no type in particular. */
const allTypes = "";

export interface IssueTypeChooserProps {
  /** What the choice is for, as its label. */
  readonly label: string;
  /** The ID of the issue type that is open; nothing for all types. */
  readonly type: string | undefined;
  readonly onChange: (type: string | undefined) => void;
}

/** Which issue type a place-wide thing is for (a layout, a workflow, field permissions): one, or all of them. */
export function IssueTypeChooser({ label, type, onChange }: IssueTypeChooserProps) {
  const types = issueTypeLinesOf(useIssueTypes().data?.pages);
  return (
    <SelectField
      label={label}
      choices={[
        { value: allTypes, label: t("issue-types.all-types") },
        ...types.map((each) => ({ value: each.id, label: each.name })),
      ]}
      value={type ?? allTypes}
      onChange={(next) => {
        onChange(next === allTypes ? undefined : next);
      }}
    />
  );
}
