import { t } from "@bugaboo/messages";
import { StatusButton, StatusMenu, partOf, part } from "@bugaboo/ui";

import { useShortcuts } from "../../shared/use-shortcuts.ts";
import { openStatusMenu, statusId } from "./bar-targets.ts";
import { readOnlyReason } from "./read-only.ts";
import type { IssueBlockProps } from "./slots.ts";
import type { IssueChanges } from "./use-issue-changes.ts";

export interface StatusChangeProps {
  readonly blocks: IssueBlockProps;
  readonly changes: IssueChanges;
}

/**
 * The big status button under the title (UX §4.4): the current status with its group's icon and
 * color, and the list of the changes the person can make. A person who may not change it sees the
 * button as it is, and why. The key `s` and the bar of a phone open the list.
 */
export function StatusChange({ blocks, changes }: StatusChangeProps) {
  const { issue, mask, form } = blocks;
  const { status } = issue;
  const canChange = mask.changes("status");
  useShortcuts(canChange ? { "change-status": openStatusMenu } : {});
  if (status === undefined || status === null || !form.shows("status")) {
    return null;
  }
  const { name: word, group } = status;
  if (!canChange) {
    return (
      <div {...partOf(part.issueStatusLine)}>
        <StatusButton
          isDisabled
          group={group}
          label={word}
          id={statusId}
          onPress={openStatusMenu}
        />
        <span {...partOf(part.muted)}>{readOnlyReason(t("issue-field.status"))}</span>
      </div>
    );
  }
  return (
    <div {...partOf(part.issueStatusLine)}>
      <StatusMenu
        group={group}
        label={word}
        id={statusId}
        menuLabel={t("issues.change-status")}
        choices={(issue.status_changes ?? []).map((change) => ({
          id: change.to.key,
          label: change.to.name,
          onAction: () => {
            changes.change({ status: change.to });
          },
        }))}
      />
    </div>
  );
}
