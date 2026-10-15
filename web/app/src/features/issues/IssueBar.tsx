import { t } from "@bugaboo/messages";
import { ActionMenu, Button, partOf, part, type MenuAction } from "@bugaboo/ui";
import { Toolbar } from "react-aria-components";

export interface IssueBarProps {
  /** Takes the person to the status of the issue to change it. */
  readonly onStatus: () => void;
  /** Takes the person to the box for a new comment. */
  readonly onComment: () => void;
  readonly actions: readonly MenuAction[];
}

/** On a phone (UX §3.3): the bar with the most used actions of an issue, *Status · Comment · More*. */
export function IssueBar({ onStatus, onComment, actions }: IssueBarProps) {
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
      <div {...partOf(part.issueBarItem)}>
        <ActionMenu label={t("issues.more")} actions={actions} />
      </div>
    </Toolbar>
  );
}
