import { t } from "@bugaboo/messages";
import type { MenuAction } from "@bugaboo/ui";
import { use } from "react";
import { createPortal } from "react-dom";

import { openStatusMenu } from "./bar-targets.ts";
import { BarSlot, useHideAppBar } from "../shell/index.ts";
import { entryIssueOf } from "./entry-issue.ts";
import { historyId } from "./history-id.ts";
import { IssueBar } from "./IssueBar.tsx";
import type { IssueView } from "./slots.ts";

/** Scrolls a part of the page into view, as the bar of a phone does; gives the part, if it is there. */
function goTo(id: string): HTMLElement | null {
  const part = document.querySelector<HTMLElement>(`#${id}`);
  part?.scrollIntoView({ block: "start" });
  return part;
}

/** The bar of the most used actions of an open issue on a phone: the status, a comment, the timer, *More*. */
export function PhoneBar({
  actions,
  issue,
}: {
  readonly actions: readonly MenuAction[];
  readonly issue: IssueView;
}) {
  // The bar of the app steps aside: this bar is the only one under an open issue (M5-F172).
  useHideAppBar();
  // The bar stands in the slot of the column of the app, under the page, and not over it.
  const slot = use(BarSlot);
  if (slot === null) {
    return null;
  }
  // A landmark of its own, so the bar is not outside every landmark of the page.
  return createPortal(
    <section aria-label={t("issues.bar")}>
      <IssueBar
        onStatus={() => {
          // The same as the key for the status: a status the person may not change has no list.
          openStatusMenu();
        }}
        onComment={() => {
          goTo(historyId)?.querySelector("textarea")?.focus();
        }}
        actions={actions}
        issue={entryIssueOf(issue)}
      />
    </section>,
    slot,
  );
}
