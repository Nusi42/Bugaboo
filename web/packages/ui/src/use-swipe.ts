import { useRef, useState, type PointerEvent } from "react";

import type { MenuAction } from "./ActionMenu.tsx";
import { sideOfSwipe, swipeSide, type FingerAt, type SwipeSide } from "./swipe.ts";

/** The actions at one side of a row: one or two, chosen per list (UX §3.3). */
export type SideActions = readonly [MenuAction] | readonly [MenuAction, MenuAction];

/** The first two of a list's actions for one side, the most a side holds; none when there are none. */
export function sideActions(actions: readonly MenuAction[]): SideActions | undefined {
  const [first, second] = actions;
  if (first === undefined) {
    return undefined;
  }
  return second === undefined ? [first] : [first, second];
}

/** The actions a list gives a row: those a swipe pulls in at the start edge and at the end edge. */
export interface RowActions {
  readonly start?: SideActions | undefined;
  readonly end?: SideActions | undefined;
}

/**
 * What a row needs to be swiped: the side that is open now, and the handlers of the finger for the row's element. They
 * listen on the way down, so a link or a button inside the row that keeps the press to itself still lets the row see
 * the finger.
 */
export interface Swipe {
  readonly side: SwipeSide;
  /**
   * The row was touched or took the focus: a list whose actions need a read of their own (whether the person follows
   * the issue) reads it from now on, not for every row it shows.
   */
  readonly awake: boolean;
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
    ...(actions.start === undefined ? [] : [swipeSide.start]),
    ...(actions.end === undefined ? [] : [swipeSide.end]),
  ];
}

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
 * never swipes (it has *More*). Once a swipe is decided the press of the row is called off.
 */
export function useSwipe(sides: readonly SwipeSide[]): Swipe {
  const [side, setSide] = useState<SwipeSide>(swipeSide.none);
  const [awake, setAwake] = useState(false);
  const from = useRef<FingerAt | null>(null);
  const forget = (): void => {
    from.current = null;
  };
  return {
    side,
    awake,
    close: () => {
      setSide(swipeSide.none);
    },
    rowProps: {
      onFocus: () => {
        setAwake(true);
      },
      onPointerDownCapture: (event) => {
        setAwake(true);
        from.current =
          event.pointerType === "mouse" ? null : { x: event.clientX, y: event.clientY };
      },
      onPointerMoveCapture: (event) => {
        const start = from.current;
        const next =
          start === null
            ? undefined
            : sideOfSwipe({
                from: start,
                to: { x: event.clientX, y: event.clientY },
                rightToLeft: getComputedStyle(event.currentTarget).direction === "rtl",
                open: side,
                sides,
              });
        if (next === undefined) {
          return;
        }
        forget();
        setSide(next);
        callOffPress(event);
      },
      onPointerUpCapture: forget,
      onPointerCancelCapture: forget,
    },
  };
}
