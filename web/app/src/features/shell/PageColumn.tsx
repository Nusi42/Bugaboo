import { Alert, alertTone, partOf, part } from "@bugaboo/ui";
import { Outlet } from "@tanstack/react-router";

import { messageOf } from "../../shared/errors.ts";
import { useSidebarNarrow } from "./sidebar-state.ts";
import { UndoHost } from "./UndoHost.tsx";

export interface PageColumnProps {
  /** On a phone the offers to undo stand under the page, in `PhoneBars`. */
  readonly phone: boolean;
  /** Why signing out did not work, if it did not. */
  readonly signOutProblem: unknown;
}

/**
 * The column the pages are drawn in. On a computer and a tablet the offers to undo are the last
 * thing in it, so they stand in the middle of the column wherever the sidebar and the window put it.
 */
export function PageColumn({ phone, signOutProblem }: PageColumnProps) {
  // The focus view narrows the sidebar to icons, and the page then uses the room that gives.
  const { locked } = useSidebarNarrow();
  return (
    <>
      <div {...partOf(part.appLayoutInner, { variants: [locked && "wide"] })}>
        {signOutProblem === null ? null : (
          <Alert tone={alertTone.danger}>{messageOf(signOutProblem)}</Alert>
        )}
        <Outlet />
      </div>
      {phone ? null : <UndoHost wide={locked} />}
    </>
  );
}
