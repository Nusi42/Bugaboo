import { t } from "@bugaboo/messages";

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
 * The fields of the Details that change in place (UX §4.4): *Assigned to* and *Priority*, each
 * saving at once and saying *Saved* with *Undo* in its own row. A field the person may not see, or
 * that the layout does not show them, is left out; one they may see and not change says why.
 */
export function ChangeableRows({ blocks, changes }: ChangeableRowsProps) {
  const { issue, mask, form } = blocks;
  const priority = known(issue.priority);
  return (
    <>
      {known(issue.assigned_to) === undefined || !form.shows("assigned-to") ? null : (
        <AssignedRow
          issue={issue}
          mask={mask}
          status={rowStatus(changes, "assigned-to")}
          onChange={changes.change}
        />
      )}
      {priority === undefined || !form.shows("priority") ? null : (
        <ChoiceRow
          name={t("issue-field.priority")}
          choices={priorityChoices}
          value={priority}
          canChange={mask.changes("priority")}
          status={rowStatus(changes, "priority")}
          onPick={(picked) => {
            changes.change({ priority: picked });
          }}
        />
      )}
    </>
  );
}
