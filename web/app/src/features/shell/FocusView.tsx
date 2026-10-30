import { useLayout, partOf, part } from "@bugaboo/ui";
import { useState, type ReactNode } from "react";

import { useShortcuts } from "../../shared/use-shortcuts.ts";
import { FocusBar } from "./FocusBar.tsx";
import { useNarrowSidebarWhile } from "./sidebar-state.ts";
import { escapeInField, useEscape } from "./use-escape.ts";
import { useTakeFocus } from "./use-take-focus.ts";

export interface FocusViewProps {
  /** A slim list of the items around the open one; `j` and `k` move through it. */
  readonly list: ReactNode;
  /** Back to the full list: also what Esc does. */
  readonly onBack: () => void;
  /** Opens the next (`1`) or the previous (`-1`) item of the list. */
  readonly onStep: (direction: 1 | -1) => void;
  /** The open item. */
  readonly children: ReactNode;
}

/**
 * The focus view (UX §3.4): the open item takes the main area; the sidebar shrinks to icons and a
 * slim list stays on the left, which *Hide list* gives up for even more room. `j` and `k` (and the
 * *Previous* and *Next* buttons) move through the list and Esc, or *Back to list*, returns to it.
 * Phones always show the item on a screen of its own, with *Back* in the top bar of every screen.
 */
export function FocusView({ list, onBack, onStep, children }: FocusViewProps) {
  const layout = useLayout();
  const [listHidden, setListHidden] = useState(false);
  useNarrowSidebarWhile(layout === "computer");
  useShortcuts({
    "list-next": () => {
      onStep(1);
    },
    "list-previous": () => {
      onStep(-1);
    },
  });
  const view = useEscape<HTMLDivElement>(onBack, escapeInField.leave);
  const main = useTakeFocus<HTMLDivElement>();
  const phone = layout === "phone";
  const listState = phone ? "none" : listHidden ? "hidden" : "shown";
  return (
    <div ref={view} {...partOf(part.focusView)}>
      {listState === "shown" ? <div {...partOf(part.focusViewList)}>{list}</div> : null}
      <div ref={main} tabIndex={-1} {...partOf(part.shellContent)}>
        <div {...partOf(part.focusViewBar)}>
          <FocusBar
            onBack={phone ? undefined : onBack}
            onStep={onStep}
            list={listState}
            onToggleList={() => {
              setListHidden(!listHidden);
            }}
          />
        </div>
        {children}
      </div>
    </div>
  );
}
