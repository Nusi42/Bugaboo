import type { MenuAction } from "@bugaboo/ui";
import { use } from "react";
import { createPortal } from "react-dom";

import { openStatusMenu } from "./bar-targets.ts";
import { BarSlot } from "../shell/index.ts";
import { historyId } from "./history-id.ts";
import { IssueBar } from "./IssueBar.tsx";

/** Scrolls a part of the page into view, as the bar of a phone does; gives the part, if it is there. */
function goTo(id: string): HTMLElement | null {
  const part = document.querySelector<HTMLElement>(`#${id}`);
  part?.scrollIntoView({ block: "start" });
  return part;
}

/** The bar of the most used actions of an open issue on a phone: the status, a comment, *More*. */
export function PhoneBar({ actions }: { readonly actions: readonly MenuAction[] }) {
  // The bar stands in the slot of the column of the app, under the page, and not over it.
  const slot = use(BarSlot);
  if (slot === null) {
    return null;
  }
  return createPortal(
    <IssueBar
      onStatus={() => {
        // The same as the key for the status: a status the person may not change has no list.
        openStatusMenu();
      }}
      onComment={() => {
        goTo(historyId)?.querySelector("textarea")?.focus();
      }}
      actions={actions}
    />,
    slot,
  );
}
