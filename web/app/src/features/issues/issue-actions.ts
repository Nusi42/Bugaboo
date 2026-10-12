import { t } from "@bugaboo/messages";
import type { MenuAction } from "@bugaboo/ui";

import { issueAddress } from "../../shared/paths.ts";
import type { IssueView } from "./slots.ts";
import { deleteIssue } from "./use-delete-issue.ts";

/** The link to share: the whole address of the issue. */
export function linkTo(key: string): string {
  return `${globalThis.location.origin}${issueAddress(key)}`;
}

/** What *More* offers: the link, and, for somebody who may, the trash. */
export function moreActions(
  issue: IssueView,
  mayDelete: boolean,
  onClose: () => void,
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
      deleteIssue(issue, onClose);
    },
  };
  return mayDelete ? [copy, remove] : [copy];
}

/** A clipboard that refuses is nothing to tell the person: the link is in the address bar. */
function ignoreRefusal(): void {
  // Nothing to do.
}

/** Puts the link on the clipboard (the menu item of *More*); a clipboard that refuses is not an error to show. */
function copyLink(key: string): void {
  void navigator.clipboard.writeText(linkTo(key)).catch(ignoreRefusal);
}
