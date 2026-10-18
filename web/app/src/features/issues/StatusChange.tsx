import { t } from "@bugaboo/messages";
import { StatusButton, StatusMenu, partOf, part, type StatusGroup } from "@bugaboo/ui";
import { useState } from "react";

import { useShortcuts } from "../../shared/use-shortcuts.ts";
import { statusActions, type OfferedChange } from "./ask-for.ts";
import { AskFor } from "./AskFor.tsx";
import { openStatusMenu, statusId } from "./bar-targets.ts";
import { readOnlyReason } from "./read-only.ts";
import type { IssueBlockProps } from "./slots.ts";
import type { IssueChanges } from "./use-issue-changes.ts";

/** The status button of a person who may not change the status: it says why. */
function StatusLocked({ group, label }: { readonly group: StatusGroup; readonly label: string }) {
  return (
    <div {...partOf(part.issueStatusLine)}>
      <StatusButton isDisabled group={group} label={label} id={statusId} onPress={openStatusMenu} />
      <span {...partOf(part.muted)}>{readOnlyReason(t("issue-field.status"))}</span>
    </div>
  );
}

export interface StatusChangeProps {
  readonly blocks: IssueBlockProps;
  readonly changes: IssueChanges;
}

/**
 * The big status button under the title (UX §4.4): the current status with its group's icon and
 * color, and the list of the status changes the person can make now, in the words of their buttons
 * (*Start working*). A change that asks for something first opens *Ask for*; any other is made at
 * once. A person who may not change it sees the button as it is, and why. The key `s` and the bar
 * of a phone open the list.
 */
export function StatusChange({ blocks, changes }: StatusChangeProps) {
  const { issue, mask, form } = blocks;
  const { status } = issue;
  const canChange = mask.changes("status");
  const [asking, setAsking] = useState<OfferedChange | null>(null);
  useShortcuts(canChange ? { "change-status": openStatusMenu } : {});
  if (status === undefined || status === null || !form.shows("status")) {
    return null;
  }
  const { name: word, group } = status;
  if (!canChange) {
    return <StatusLocked group={group} label={word} />;
  }
  return (
    <div {...partOf(part.issueStatusLine)}>
      <StatusMenu
        group={group}
        label={word}
        id={statusId}
        menuLabel={t("issues.change-status")}
        cancelLabel={t("common.cancel")}
        choices={statusActions(issue.status_changes ?? [], {
          ask: setAsking,
          make: (change) => {
            changes.change({ status: change.to });
          },
        })}
      />
      {asking === null ? null : (
        <AskFor
          issue={issue}
          change={asking}
          changes={changes}
          onClose={() => {
            setAsking(null);
          }}
        />
      )}
    </div>
  );
}
