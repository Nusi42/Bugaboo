import { t } from "@bugaboo/messages";
import type { MenuAction } from "@bugaboo/ui";
import { useState, type ReactNode } from "react";

import { messageOf } from "../../shared/errors.ts";
import { issueAddress } from "../../shared/paths.ts";
import { moreEntries } from "./more-entries.ts";
import type { IssueView } from "./slots.ts";
import { deleteIssue, type DeleteHandlers } from "./use-delete-issue.ts";

/** The link to share: the whole address of the issue. */
export function linkTo(key: string): string {
  return `${globalThis.location.origin}${issueAddress(key)}`;
}

/** What *More* offers: the link, and, for somebody who may, the trash. */
export function moreActions(
  issue: IssueView,
  mayDelete: boolean,
  handlers: DeleteHandlers,
): MenuAction[] {
  const key = issue.readable_id ?? "";
  const copy = {
    id: "copy",
    label: t("issues.copy-link"),
    onAction: () => {
      copyLink(key);
    },
  };
  const remove = {
    id: "delete",
    label: t("issues.delete"),
    onAction: () => {
      deleteIssue(issue, handlers);
    },
  };
  return mayDelete ? [copy, remove] : [copy];
}

/** The same action, which first forgets the reason of the last refusal. */
function withoutRefusal(action: MenuAction, forget: () => void): MenuAction {
  return {
    id: action.id,
    label: action.label,
    onAction: () => {
      forget();
      action.onAction();
    },
  };
}

/**
 * What *More* offers for an issue, the window of the entry that is open, and what to tell when a
 * delete is refused: the page that is still open says it.
 */
export function useMoreActions(
  issue: IssueView,
  mayDelete: boolean,
  onClose: () => void,
): {
  readonly actions: MenuAction[];
  readonly refusal: string | null;
  readonly dialog: ReactNode;
} {
  const [refusal, setRefusal] = useState<string | null>(null);
  const [open, setOpen] = useState<string | null>(null);
  const offered = moreActions(issue, mayDelete, {
    onDeleted: onClose,
    onRefused: (error) => {
      setRefusal(messageOf(error));
    },
  });
  // What other packages add comes before the trash, which stays last.
  const added: MenuAction[] = moreEntries().map((entry) => ({
    id: entry.id,
    label: entry.label(),
    onAction: () => {
      setOpen(entry.id);
    },
  }));
  const [copy, ...rest] = offered;
  const all = copy === undefined ? offered : [copy, ...added, ...rest];
  // Whatever is asked next starts without the reason of the last refusal.
  const actions = all.map((action) =>
    withoutRefusal(action, () => {
      setRefusal(null);
    }),
  );
  const entry = moreEntries().find((known) => known.id === open);
  const dialog =
    entry === undefined
      ? null
      : entry.dialog({
          issue,
          onClose: () => {
            setOpen(null);
          },
        });
  return { actions, refusal, dialog };
}

/** A clipboard that refuses is nothing to tell the person: the link is in the address bar. */
function ignoreRefusal(): void {
  // Nothing to do.
}

/** Puts the link on the clipboard (the menu item of *More*); a clipboard that refuses is not an error to show. */
function copyLink(key: string): void {
  void navigator.clipboard.writeText(linkTo(key)).catch(ignoreRefusal);
}
