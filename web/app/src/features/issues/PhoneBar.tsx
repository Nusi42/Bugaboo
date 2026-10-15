import type { MenuAction } from "@bugaboo/ui";

import { focusChoice } from "./focus-choice.ts";
import { statusId } from "./bar-targets.ts";
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
  return (
    <IssueBar
      onStatus={() => {
        // The same lookup as the key for the status: a status the person may not change has no list.
        focusChoice(goTo(statusId));
      }}
      onComment={() => {
        goTo(historyId)?.querySelector("textarea")?.focus();
      }}
      actions={actions}
    />
  );
}
