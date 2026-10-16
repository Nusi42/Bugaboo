import { t } from "@bugaboo/messages";
import { StatusButton, StatusMenu, partOf, part } from "@bugaboo/ui";

import { known } from "../../shared/present.ts";
import { useShortcuts } from "../../shared/use-shortcuts.ts";
import { openStatusMenu, statusId } from "./bar-targets.ts";
import { statusChoices } from "./issue-choices.ts";
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
  const [status, group] = [known(issue.status), known(issue.status_group)];
  const canChange = mask.changes("status");
  useShortcuts(canChange ? { "change-status": openStatusMenu } : {});
  if (status === undefined || group === undefined || !form.shows("status")) {
    return null;
  }
  const word = t(`status.${status}`);
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
        choices={statusChoices
          .filter((choice) => choice.value !== status)
          .map((choice) => ({
            id: choice.value,
            label: choice.label,
            onAction: () => {
              changes.change({ status: choice.value });
            },
          }))}
      />
    </div>
  );
}
