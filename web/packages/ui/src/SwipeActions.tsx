import { useRef } from "react";

import { ActionMenu } from "./ActionMenu.tsx";
import { Announce } from "./Announce.tsx";
import { partOf } from "./parts.ts";
import { MoreButton } from "./MoreButton.tsx";
import { OpenSide } from "./OpenSide.tsx";
import { MOST_SWIPED, swipeSide } from "./swipe.ts";
import { part } from "./tokens.ts";
import type { RowActions, Swipe } from "./use-swipe.ts";

export interface SwipeActionsProps {
  readonly actions: RowActions;
  readonly swipe: Swipe;
  /** The name of the row's *More*, with what the row is: *More: WEB-12*. */
  readonly moreLabel: string;
  /** The word of *Cancel* on the sheet of *More*. */
  readonly cancelLabel: string;
  /** What a screen reader hears when a swipe brought these actions in (*Actions: Follow*). */
  readonly announce: (names: readonly string[]) => string;
}

/**
 * The swipe actions of a row of a list (UX §3.3, UX-02): the side a swipe opened, and the row's *More*, which holds
 * every action of both sides, so nothing depends on a swipe (principle 9, TEST-A11Y). The row spreads
 * `swipe.rowProps` on its own element; *More* stands where the row puts this component, the open side under it.
 */
export function SwipeActions({
  actions,
  swipe,
  moreLabel,
  cancelLabel,
  announce,
}: SwipeActionsProps) {
  const more = useRef<HTMLSpanElement>(null);
  const every = [...(actions.start ?? []), ...(actions.end ?? [])];
  const open = (swipe.side === swipeSide.none ? [] : (actions[swipe.side] ?? [])).slice(
    0,
    MOST_SWIPED,
  );
  return (
    <>
      {open.length === 0 || swipe.side === swipeSide.none ? null : (
        <OpenSide side={swipe.side} actions={open} close={swipe.close} more={more} />
      )}
      {every.length === 0 ? null : (
        <span ref={more} {...partOf(part.rowMore)}>
          <ActionMenu
            label={moreLabel}
            actions={every}
            cancelLabel={cancelLabel}
            trigger={MoreButton}
          />
        </span>
      )}
      <Announce>{open.length === 0 ? "" : announce(open.map((action) => action.label))}</Announce>
    </>
  );
}
