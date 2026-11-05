import { t } from "@bugaboo/messages";
import {
  ActionMenu,
  Icon,
  iconName,
  partOf,
  part,
  type ActionMenuTriggerProps,
  type IconName,
  type MenuAction,
} from "@bugaboo/ui";
import type { ReactNode } from "react";
import { Button, Toolbar } from "react-aria-components";

import { TimerBarButton, TimerNotice, type EntryIssue } from "../time/index.ts";

export interface IssueBarProps {
  /** Takes the person to the status of the issue to change it. */
  readonly onStatus: () => void;
  /** Takes the person to the box for a new comment. */
  readonly onComment: () => void;
  readonly actions: readonly MenuAction[];
  /** The issue, for the timer: none when the person may not book time on it or the page does not know it. */
  readonly issue: EntryIssue | undefined;
}

/** A place of the bar: an icon over its words, as the places of the bar of the app (UX §3.3). */
function Place({ icon, children }: { readonly icon: IconName; readonly children: ReactNode }) {
  return (
    <>
      <span {...partOf(part.bottomBarLabel)}>{children}</span>
      <span {...partOf(part.bottomBarIcon)}>
        <Icon name={icon} />
      </span>
    </>
  );
}

/** The button that opens *More*. */
function MoreButton({ label, onOpen }: ActionMenuTriggerProps) {
  return (
    <Button {...partOf(part.bottomBarItem)} {...(onOpen === undefined ? {} : { onPress: onOpen })}>
      <Place icon={iconName.more}>{label}</Place>
    </Button>
  );
}

/**
 * On a phone (UX §3.3): the bar with the most used actions of an issue, *Status · Comment · Timer ·
 * More*, drawn as the bar of the app is: an icon over its words, no boxes. It is the only bar while
 * an issue is open.
 */
export function IssueBar({ onStatus, onComment, actions, issue }: IssueBarProps) {
  return (
    <>
      {/* What the timer tells (*Saved 42m on WEB-142.*) stands over the bar: the bar of the app is hidden here. */}
      <TimerNotice />
      <Toolbar {...partOf(part.issueBar)} aria-label={t("issues.bar")}>
        <div {...partOf(part.issueBarItem)}>
          <Button {...partOf(part.bottomBarItem)} onPress={onStatus}>
            <Place icon={iconName.status}>{t("issue-field.status")}</Place>
          </Button>
        </div>
        <div {...partOf(part.issueBarItem)}>
          <Button {...partOf(part.bottomBarItem)} onPress={onComment}>
            <Place icon={iconName.comment}>{t("issues.comment")}</Place>
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
            trigger={MoreButton}
          />
        </div>
      </Toolbar>
    </>
  );
}
