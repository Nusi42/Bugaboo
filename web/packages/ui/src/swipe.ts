/**
 * The sides of a row whose actions a swipe shows (UX §3.3): none, the actions at its start edge, or those at its end
 * edge. A finger that moves towards the end edge pulls the start's actions in, one that moves towards the start pulls
 * the end's.
 */
export const swipeSide = { none: "none", start: "start", end: "end" } as const;
export type SwipeSide = (typeof swipeSide)[keyof typeof swipeSide];

/** The most actions a swipe brings in at one side (UX §3.3); *More* holds every action of the row. */
export const MOST_SWIPED = 2;

/**
 * How far a finger moves sideways before a swipe opens or closes a side: about the width of a fingertip, so a tap
 * that wobbles is still a tap.
 */
export const SWIPE_DISTANCE_PX = 48;

/** Where a finger is, in pixels of the page. */
export interface FingerAt {
  readonly x: number;
  readonly y: number;
}

/** What a swipe decides from: where the finger started, where it is, the writing direction and the open side. */
export interface SwipeMove {
  readonly from: FingerAt;
  readonly to: FingerAt;
  /** The page is written from right to left: the start edge is on the right. */
  readonly rightToLeft: boolean;
  readonly open: SwipeSide;
  /** The sides that have actions. */
  readonly sides: readonly SwipeSide[];
}

/**
 * The side a move opens, `none` when it closes the open one, or nothing while it decides nothing yet: it is still
 * short, or it goes more up or down than sideways (the list scrolls), or it pulls in a side that has no actions.
 */
export function sideOfSwipe({
  from,
  to,
  rightToLeft,
  open,
  sides,
}: SwipeMove): SwipeSide | undefined {
  const across = to.x - from.x;
  if (Math.abs(across) < SWIPE_DISTANCE_PX || Math.abs(to.y - from.y) > Math.abs(across)) {
    return undefined;
  }
  const towardsEnd = rightToLeft ? across < 0 : across > 0;
  const pulled = towardsEnd ? swipeSide.start : swipeSide.end;
  if (open !== swipeSide.none) {
    return pulled === open ? undefined : swipeSide.none;
  }
  return sides.includes(pulled) ? pulled : undefined;
}
