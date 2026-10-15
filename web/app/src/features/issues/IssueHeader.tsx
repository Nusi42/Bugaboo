import { t } from "@bugaboo/messages";
import {
  ActionMenu,
  Button,
  buttonLook,
  CopyButton,
  InPlaceText,
  IssueTypeIcon,
  type MenuAction,
  partOf,
  part,
} from "@bugaboo/ui";

import { known } from "../../shared/present.ts";
import { linkTo } from "./issue-actions.ts";
import type { IssueShow } from "./issue-show.ts";
import type { IssueView } from "./slots.ts";

export interface IssueHeaderProps {
  readonly issue: IssueView;
  readonly show: IssueShow;
  /** Whether the person may change the title. */
  readonly canChangeTitle: boolean;
  /** What *More* offers. */
  readonly actions: readonly MenuAction[];
  readonly onTitle: (title: string) => void;
  /** Close the panel, or go back to the list. */
  readonly onClose: () => void;
  /** Open in the other way: from the panel to the full page, from the focus view to the panel. */
  readonly onSwitch: () => void;
}

/**
 * The top of an issue (UX §4.4): its type, its ID that copies a link, the title that is changed
 * where it stands, *More*, the way to open it in the other view, and *Close*.
 */
export function IssueHeader(props: IssueHeaderProps) {
  const { issue, show, canChangeTitle, actions, onTitle, onClose, onSwitch } = props;
  const key = issue.key ?? "";
  const issueType = known(issue.issue_type);
  const Heading = show === "focus" ? "h1" : "h2";
  return (
    <>
      <div {...partOf(part.issueHead)}>
        <span {...partOf(part.issueHeadKind)}>
          {issueType === undefined ? null : (
            <>
              <IssueTypeIcon name={issueType.icon} />
              {issueType.name}
            </>
          )}
        </span>
        <CopyButton text={linkTo(key)} label={key} copiedLabel={t("issues.link-copied")} />
        <div {...partOf(part.issueHeadActions)}>
          <Button onPress={onSwitch}>
            {show === "focus" ? t("issues.side-panel-view") : t("issues.open-full-page")}
          </Button>
          <ActionMenu label={t("issues.more")} actions={actions} />
          {show === "panel" ? (
            <Button look={buttonLook.quiet} onPress={onClose}>
              {t("common.close")}
            </Button>
          ) : null}
        </div>
      </div>
      <Heading {...partOf(part.issueTitle)}>
        <InPlaceText
          label={t("issue-field.title")}
          changeLabel={t("issues.change-title")}
          value={issue.title ?? ""}
          required
          readOnly={!canChangeTitle}
          onSave={onTitle}
        />
      </Heading>
    </>
  );
}
