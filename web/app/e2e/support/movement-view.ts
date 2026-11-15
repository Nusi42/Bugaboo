import { differs, far, LEAST_MOVE_PX, type Movement, type Moved } from "./movement-frames.ts";
import type { Box } from "./movement-record.ts";

// What of a movement the person sees in the window (UX §6.4): the browser keeps what is in view still by its scroll
// when something above it changes, so the measure of movement (`movement-judge.ts`) judges what is in view.

/**
 * The parts that moved where their edges show in view, and whether anything changed out of view only (a part, a row
 * that came in above the view). What stands outside the
 * window at the start and at the end is not in view: the browser keeps what is in view still when
 * something above it grows, by its scroll (UX §6.4).
 */
export function inViewOf(movement: Movement): {
  readonly seen: readonly Moved[];
  readonly anchored: boolean;
} {
  const { view } = movement;
  const inView = (box: Box): boolean =>
    view === undefined || (box.y < view && box.y + box.height > 0);
  // A part is judged by its edges that show in the window: one that changed only where it is out of view (grew above
  // it, its foot in view kept still by the scroll) looks the same to the person. A part with neither edge in view is
  // judged whole.
  const looksMoved = ({ from, last }: Moved): boolean => {
    if (view === undefined || !inView(from) || !inView(last)) {
      return true;
    }
    const topSeen = from.y >= 0 && last.y >= 0;
    const footSeen = from.y + from.height <= view && last.y + last.height <= view;
    if (far(from.x, last.x) || far(from.width, last.width) || (!topSeen && !footSeen)) {
      return differs(from, last);
    }
    return (
      (topSeen && far(from.y, last.y)) ||
      (footSeen && far(from.y + from.height, last.y + last.height))
    );
  };
  const seen = movement.moved.filter(
    (part) => (inView(part.from) || inView(part.last)) && looksMoved(part),
  );
  // Something changed out of view: a part, or a row that came in above it.
  const above = movement.appeared.some((part) => part.last.y + part.last.height <= 0);
  return { seen, anchored: above || seen.length < movement.moved.length };
}

/**
 * How far the one box that scrolled was scrolled down, if exactly one did (the browser keeping the view still), and
 * where that box stands.
 */
function keptByScroll(movement: Movement): { readonly by: number; readonly box: Box | undefined } {
  const [only, ...more] = movement.scrolled;
  if (only === undefined || more.length > 0) {
    return { by: 0, box: undefined };
  }
  return { by: scrollDownOf(only.to) - scrollDownOf(only.from), box: only.box };
}

/** Whether one box holds another, to the half pixel. */
function holdsBox(outer: Box, inner: Box): boolean {
  return (
    inner.x >= outer.x - LEAST_MOVE_PX &&
    inner.y >= outer.y - LEAST_MOVE_PX &&
    inner.x + inner.width <= outer.x + outer.width + LEAST_MOVE_PX &&
    inner.y + inner.height <= outer.y + outer.height + LEAST_MOVE_PX
  );
}

/** How far down a part is scrolled, as the recording writes its scroll (`40/0`). */
function scrollDownOf(at: string): number {
  return Number(at.split("/")[0]);
}

/**
 * A part with neither edge in view that grew by at least the scroll, set back by the scroll that kept the view still:
 * it is judged where it stands in its box, as the person sees what of it is in view (UX §6.4, the browser keeps what is
 * in view still by its scroll). Only a scroll its growth explains, while a part in view stood still, is set back; any
 * other stays wrong.
 */
function inTheBox(part: Moved, movement: Movement): Moved {
  const { view } = movement;
  const spans = (box: Box): boolean => view !== undefined && box.y < 0 && box.y + box.height > view;
  const { by, box: scroller } = keptByScroll(movement);
  // The part changed by at least the scroll, the same way (it grew above the view, or shrank there), and a part in
  // view inside the box that scrolled, not spanning the view, stood still: the scroll kept what is in view, rather than
  // moving it. A part outside that box (a menu beside it, a head above it) stands still whatever the box does.
  const grew = part.last.height - part.from.height;
  const explains = by > 0 ? grew >= by - LEAST_MOVE_PX : grew <= by + LEAST_MOVE_PX;
  const kept = (movement.still ?? []).some(
    (box) =>
      view !== undefined &&
      scroller !== undefined &&
      holdsBox(scroller, box) &&
      box.y < view &&
      box.y + box.height > 0 &&
      !spans(box),
  );
  if (by === 0 || !spans(part.from) || !spans(part.last) || !explains || !kept) {
    return part;
  }
  return {
    ...part,
    to: { ...part.to, y: part.to.y + by },
    last: { ...part.last, y: part.last.y + by },
  };
}

/** The measure with each part that spans the view set back by the scroll, and whether any was. */
export function inTheBoxes(measured: Movement): {
  readonly movement: Movement;
  readonly setBack: boolean;
} {
  const moved = measured.moved.map((part) => inTheBox(part, measured));
  // A part set back by the scroll changed out of view: the scroll kept what is in view still.
  const setBack = moved.some((part, index) => part !== measured.moved[index]);
  return { movement: { ...measured, moved }, setBack };
}
