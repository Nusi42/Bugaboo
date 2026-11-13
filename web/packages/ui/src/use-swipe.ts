import { useRef, useState, type PointerEvent } from "react";

import type { MenuAction } from "./ActionMenu.tsx";
import { sideOfSwipe, swipeSide, type FingerAt, type SwipeSide } from "./swipe.ts";

/**
 * The actions a list gives a row (UX §3.3): those at its start edge and those at its end edge. *More* holds every one
 * of them; a swipe brings in the first `MOST_SWIPED` of a side.
 */
export interface RowActions {
  readonly start?: readonly MenuAction[] | undefined;
  readonly end?: readonly MenuAction[] | undefined;
}

/**
 * What a row needs to be swiped: the side that is open now, and the handlers of the finger for the row's element. They
 * listen on the way down, so a link or a button inside the row that keeps the press to itself still lets the row see
 * the finger.
 */
export interface Swipe {
  readonly side: SwipeSide;
  readonly close: () => void;
  readonly rowProps: {
    readonly onFocus: () => void;
    readonly onPointerDownCapture: (event: PointerEvent<HTMLElement>) => void;
    readonly onPointerMoveCapture: (event: PointerEvent<HTMLElement>) => void;
    readonly onPointerUpCapture: () => void;
    readonly onPointerCancelCapture: () => void;
  };
}

/** The sides of a row that have actions: what `useSwipe` lets a finger open. */
export function sidesOf(actions: RowActions): readonly SwipeSide[] {
  return [
    ...((actions.start ?? []).length === 0 ? [] : [swipeSide.start]),
    ...((actions.end ?? []).length === 0 ? [] : [swipeSide.end]),
  ];
}

/** Both sides: which one a finger pulls is decided first, whether it has actions after. */
const bothSides: readonly SwipeSide[] = [swipeSide.start, swipeSide.end];

/**
 * Calls off the press of the row the finger began: React Aria takes a `pointercancel` for that, wherever it comes from,
 * so lifting the finger after a swipe does not open the row as a tap would.
 */
function callOffPress(event: PointerEvent<HTMLElement>): void {
  event.currentTarget.dispatchEvent(
    new globalThis.PointerEvent("pointercancel", {
      bubbles: true,
      pointerId: event.pointerId,
      pointerType: event.pointerType,
    }),
  );
}

/**
 * Follows a finger on a row: a swipe sideways pulls in the actions of one of `sides`, the other way closes them. A mouse
 * never swipes (it has *More*). Once a swipe is decided the press of the row is called off. `onTouch` hears that the
 * row was touched or focused: a list whose actions need a read of their own (whether the person follows the issue)
 * reads it from then on, not for every row it shows.
 */
export function useSwipe(sides: readonly SwipeSide[], onTouch?: () => void): Swipe {
  const [opened, setOpened] = useState<SwipeSide>(swipeSide.none);
  // A side the finger pulled before its actions were read (whether the person follows): it opens once they are.
  const [wanted, setWanted] = useState<SwipeSide>(swipeSide.none);
  const side = opened === swipeSide.none && sides.includes(wanted) ? wanted : opened;
  const from = useRef<FingerAt | null>(null);
  const forget = (): void => {
    from.current = null;
  };
  return {
    side,
    close: () => {
      setOpened(swipeSide.none);
      setWanted(swipeSide.none);
    },
    rowProps: {
      onFocus: () => {
        onTouch?.();
      },
      onPointerDownCapture: (event) => {
        onTouch?.();
        from.current =
          event.pointerType === "mouse" ? null : { x: event.clientX, y: event.clientY };
      },
      onPointerMoveCapture: (event) => {
        const start = from.current;
        const pulled =
          start === null
            ? undefined
            : sideOfSwipe({
                from: start,
                to: { x: event.clientX, y: event.clientY },
                rightToLeft: getComputedStyle(event.currentTarget).direction === "rtl",
                open: side,
                sides: bothSides,
              });
        if (pulled === undefined) {
          return;
        }
        forget();
        callOffPress(event);
        const now = pulled === swipeSide.none || sides.includes(pulled);
        setOpened(now ? pulled : swipeSide.none);
        setWanted(now ? swipeSide.none : pulled);
      },
      onPointerUpCapture: forget,
      onPointerCancelCapture: forget,
    },
  };
}
