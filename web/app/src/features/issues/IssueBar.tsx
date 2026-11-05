import { t } from "@bugaboo/messages";
import { ActionMenu, Button, partOf, part, type MenuAction } from "@bugaboo/ui";
import { Toolbar } from "react-aria-components";

import { TimerBarButton, type EntryIssue } from "../time/index.ts";

export interface IssueBarProps {
  /** Takes the person to the status of the issue to change it. */
  readonly onStatus: () => void;
  /** Takes the person to the box for a new comment. */
  readonly onComment: () => void;
  readonly actions: readonly MenuAction[];
  /** The issue, for the timer: none when the person may not book time on it or the page does not know it. */
  readonly issue: EntryIssue | undefined;
}

/** On a phone (UX §3.3): the bar with the most used actions of an issue, *Status · Comment · Timer · More*. */
export function IssueBar({ onStatus, onComment, actions, issue }: IssueBarProps) {
  return (
    <Toolbar {...partOf(part.issueBar)} aria-label={t("issues.bar")}>
      <div {...partOf(part.issueBarItem)}>
        <Button wide onPress={onStatus}>
          {t("issue-field.status")}
        </Button>
      </div>
      <div {...partOf(part.issueBarItem)}>
        <Button wide onPress={onComment}>
          {t("issues.comment")}
        </Button>
      </div>
      {issue === undefined ? null : (
        <div {...partOf(part.issueBarItem)}>
          <TimerBarButton issue={issue} />
        </div>
      )}
      <div {...partOf(part.issueBarItem)}>
        <ActionMenu
          label={t("issues.more")}
          actions={actions}
          cancelLabel={t("common.cancel")}
          wide
        />
      </div>
    </Toolbar>
  );
}
