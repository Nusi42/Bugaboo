import { t } from "@bugaboo/messages";
import { ActionMenu, Button, partOf, part, type MenuAction } from "@bugaboo/ui";

export interface IssueBarProps {
  /** Goes to the status of the issue to change it. */
  readonly onStatus: () => void;
  /** Goes to the comments. */
  readonly onComment: () => void;
  readonly actions: readonly MenuAction[];
}

/** On a phone (UX §3.3): the bar with the most used actions of an issue, *Status · Comment · More*. */
export function IssueBar({ onStatus, onComment, actions }: IssueBarProps) {
  return (
    <nav {...partOf(part.issueBar)} aria-label={t("issues.bar")}>
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
    </nav>
  );
}
