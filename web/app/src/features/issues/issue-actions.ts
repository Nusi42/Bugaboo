import { t } from "@bugaboo/messages";
import type { MenuAction } from "@bugaboo/ui";
import { useState } from "react";

import { messageOf } from "../../shared/errors.ts";
import { issueAddress } from "../../shared/paths.ts";
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
  const key = issue.key ?? "";
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

/**
 * What *More* offers for an issue, and what to tell when a delete is refused: the page that is
 * still open says it.
 */
export function useMoreActions(
  issue: IssueView,
  mayDelete: boolean,
  onClose: () => void,
): { readonly actions: MenuAction[]; readonly refusal: string | null } {
  const [refusal, setRefusal] = useState<string | null>(null);
  const actions = moreActions(issue, mayDelete, {
    onDeleted: onClose,
    onRefused: (error) => {
      setRefusal(messageOf(error));
    },
  });
  return { actions, refusal };
}

/** A clipboard that refuses is nothing to tell the person: the link is in the address bar. */
function ignoreRefusal(): void {
  // Nothing to do.
}

/** Puts the link on the clipboard (the menu item of *More*); a clipboard that refuses is not an error to show. */
function copyLink(key: string): void {
  void navigator.clipboard.writeText(linkTo(key)).catch(ignoreRefusal);
}
