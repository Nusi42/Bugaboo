import { t } from "@bugaboo/messages";
import { sideActions, type MenuAction, type RowActions } from "@bugaboo/ui";
import { useContext, useState, type ReactNode } from "react";

import type { RowSwipe } from "../../shared/use-row-swipe.tsx";
import { useFollowing } from "../follow/index.ts";
import { RowDialogContext, rowDialogs, type OpenRowDialog, type RowIssue } from "./row-dialogs.ts";

/** *Follow* or *Stop following*, once the person's choice is read: the row reads it only after it was touched. */
function useFollowAction(issue: RowIssue, awake: boolean): MenuAction | undefined {
  const mine = useFollowing(issue.key, awake);
  return mine.known
    ? {
        id: "follow",
        label: mine.following ? t("followers.stop") : t("followers.follow"),
        onAction: mine.turn,
      }
    : undefined;
}

export interface IssueRowActionsProps {
  readonly issue: RowIssue;
  readonly swipe: RowSwipe;
}

/**
 * The actions of a row of the issue list on a phone (UX §3.3): *Follow* or *Stop following* at its start, the windows
 * other areas add (*Change status…*) at its end. A swipe brings in one side; the row's *More* holds them all.
 */
export function IssueRowActions({ issue, swipe }: IssueRowActionsProps) {
  const onRowDialog = useContext(RowDialogContext);
  const follow = useFollowAction(issue, swipe.awake);
  const actions: RowActions = {
    start: sideActions(follow === undefined ? [] : [follow]),
    end: sideActions(
      rowDialogs().map((entry) => ({
        id: entry.id,
        label: entry.label(),
        onAction: () => {
          onRowDialog({ entry, issue });
        },
      })),
    ),
  };
  return swipe.actions(actions, issue.key);
}

/** Holds the window of an action of a row for the list around it, and draws it while it is open. */
export function RowDialogs({ children }: { readonly children: ReactNode }) {
  const [open, setOpen] = useState<OpenRowDialog | null>(null);
  return (
    <RowDialogContext value={setOpen}>
      {children}
      {open === null ? null : (
        <open.entry.dialog
          issue={open.issue}
          onClose={() => {
            setOpen(null);
          }}
        />
      )}
    </RowDialogContext>
  );
}
