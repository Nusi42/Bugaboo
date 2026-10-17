import { t } from "@bugaboo/messages";
import { SelectField } from "@bugaboo/ui";

import { issueTypeLinesOf, useIssueTypes } from "./issue-types.ts";

/** What the list holds for all issue types: the word of no type in particular. */
const allTypes = "";

export interface IssueTypeChoiceProps {
  /** What the choice is for, as its label. */
  readonly label: string;
  /** The issue types to choose from. */
  readonly types: readonly { readonly id: string; readonly name: string }[];
  /** The ID of the issue type that is open; nothing for all types. */
  readonly type: string | undefined;
  readonly onChange: (type: string | undefined) => void;
}

/** One issue type of the list, or all of them: the choice itself, for types a screen already has. */
export function IssueTypeChoice({ label, types, type, onChange }: IssueTypeChoiceProps) {
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

export type IssueTypeChooserProps = Omit<IssueTypeChoiceProps, "types">;

/** Which issue type a place-wide thing is for (a layout, a workflow, field permissions): one, or all of them. */
export function IssueTypeChooser(props: IssueTypeChooserProps) {
  return <IssueTypeChoice {...props} types={issueTypeLinesOf(useIssueTypes().data?.pages)} />;
}
