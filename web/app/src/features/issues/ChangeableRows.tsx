import { t } from "@bugaboo/messages";
import { Priority, type PriorityWord } from "@bugaboo/ui";

import { known } from "../../shared/present.ts";
import { AssignedRow } from "./AssignedRow.tsx";
import { ChoiceRow } from "./ChoiceRow.tsx";
import { priorityChoices } from "./issue-choices.ts";
import { rowStatus } from "./row-status.ts";
import type { IssueBlockProps } from "./slots.ts";
import type { IssueChanges } from "./use-issue-changes.ts";

export interface ChangeableRowsProps {
  readonly blocks: IssueBlockProps;
  readonly changes: IssueChanges;
}

/**
 * The fields of the Details that change in place (UX §4.4), each saving at once and saying *Saved*
 * with *Undo* in its own row. A field the person may not see, or that the layout does not show
 * them, is left out; one they may see and not change says why. This is *Assigned to*.
 */
export function AssignedToRow({ blocks, changes }: ChangeableRowsProps) {
  const { issue, mask, form } = blocks;
  return known(issue.assigned_to) === undefined || !form.shows("assigned-to") ? null : (
    <AssignedRow
      issue={issue}
      mask={mask}
      status={rowStatus(changes, "assigned-to")}
      onChange={changes.change}
    />
  );
}

/** The icon of a priority before its word or its list (R5). */
function priorityLead(word: PriorityWord, label: string, showWord: boolean) {
  return <Priority priority={word} label={label} showWord={showWord} />;
}

/** *Priority*, the other field of the Details that changes in place. */
export function PriorityRow({ blocks, changes }: ChangeableRowsProps) {
  const { issue, mask, form } = blocks;
  const priority = known(issue.priority);
  return priority === undefined || !form.shows("priority") ? null : (
    <ChoiceRow
      name={t("issue-field.priority")}
      choices={priorityChoices}
      value={priority}
      canChange={mask.changes("priority")}
      status={rowStatus(changes, "priority")}
      lead={priorityLead}
      onPick={(picked) => {
        changes.change({ priority: picked });
      }}
    />
  );
}
