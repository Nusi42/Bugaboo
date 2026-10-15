import type { MenuAction } from "@bugaboo/ui";

import { statusId } from "./bar-targets.ts";
import { historyId } from "./history-id.ts";
import { IssueBar } from "./IssueBar.tsx";

/**
 * Takes the person to a part of the page, as the bar of a phone does: the part scrolls into view
 * and the first control inside it (if it has one) gets the focus. A part without a control, such
 * as a status the person may not change, is only scrolled to.
 */
function goTo(id: string, control: string): void {
  const part = document.querySelector<HTMLElement>(`#${id}`);
  part?.scrollIntoView({ block: "start" });
  part?.querySelector<HTMLElement>(control)?.focus();
}

/** The bar of the most used actions of an open issue on a phone: the status, a comment, *More*. */
export function PhoneBar({ actions }: { readonly actions: readonly MenuAction[] }) {
  return (
    <IssueBar
      onStatus={() => {
        goTo(statusId, "select");
      }}
      onComment={() => {
        goTo(historyId, "textarea");
      }}
      actions={actions}
    />
  );
}
