import { far, LEAST_MOVE_PX, type Movement, type Moved, type Scrolled } from "./movement-frames.ts";
import type { Box, Shift } from "./movement-record.ts";

export { judged, scrolledOf } from "./movement-frames.ts";
export type { Appeared, Movement, Moved, Scrolled } from "./movement-frames.ts";

// What the measure of movement (`movement.ts`) makes of the frames it recorded, and the rule it holds
// them to (UX §6.4). Pure, so the rule itself is tested without a browser (`movement-judge.test.ts`).

/** What may change while the screen otherwise stands still. */
export interface StillExcept {
  /** The parts the action changes: they alone may grow (at their top or bottom edge, which stays). */
  readonly grows?: readonly string[];
  /**
   * The parts that stand under a part that grows: each may be pushed once, by exactly what the parts
   * of `grows` above it in its column grew (and the rows that came in above it), never more or less.
   */
  readonly follows?: readonly string[];
  /** The parts whose scroll the action is meant to change (`window.body` when a form starts again). */
  readonly scrolls?: readonly string[];
  /**
   * The parts the action moves (a bar of the timeline moved by a day): each may take another place
   * along its line, sideways and in one direction only, at its own size, and nothing is pushed by it.
   */
  readonly moves?: readonly string[];
}

/** A part that grew, by how much, and where it stood: what pushes the parts under it. */
interface Growth {
  readonly name: string;
  readonly box: Box;
  readonly by: number;
  /** A part that came in is set against where the others stand at the end, not at the start. */
  readonly cameIn: boolean;
}

/** Whether two boxes share some of their width (stand in one column). */
function sideBySide(one: Box, other: Box): boolean {
  return one.x < other.x + other.width && other.x < one.x + one.width;
}

/** Whether one box holds the other. */
function holds(outer: Box, inner: Box): boolean {
  return (
    inner.x >= outer.x - LEAST_MOVE_PX &&
    inner.y >= outer.y - LEAST_MOVE_PX &&
    inner.x + inner.width <= outer.x + outer.width + LEAST_MOVE_PX &&
    inner.y + inner.height <= outer.y + outer.height + LEAST_MOVE_PX
  );
}

/**
 * How far the parts that grew above `part`, in its column, push it down. A part that holds another
 * part that grew (the list of the Details around the row of *Tags*) grew by that one, so it is not
 * counted again.
 */
function pushOf(part: Moved, growths: readonly Growth[]): number {
  const innermost = growths.filter(
    (growth) => !growths.some((other) => other !== growth && holds(growth.box, other.box)),
  );
  return innermost
    .filter(
      (growth) =>
        growth.name !== part.name &&
        sideBySide(growth.box, part.from) &&
        growth.box.y + growth.box.height <=
          (growth.cameIn ? part.last.y : part.from.y) + LEAST_MOVE_PX,
    )
    .reduce((sum, growth) => sum + growth.by, 0);
}

/**
 * Whether a part that changes grew in place: in one direction only, frame by frame (never shrinking
 * and growing back), with its start edge staying, as wide or wider (a button with a longer word),
 * never narrower, and its top or its bottom edge staying, or its top pushed by what grew above.
 */
function grewInPlace(part: Moved, pushed: number): boolean {
  const top = part.last.y - part.from.y;
  const bottom = top + part.last.height - part.from.height;
  return (
    part.turns === 0 &&
    !far(part.from.x, part.last.x) &&
    part.last.width >= part.from.width - LEAST_MOVE_PX &&
    (!far(top, pushed) || !far(bottom, 0))
  );
}

/** Whether a part the action moves went sideways, one way, at its own size and height on the page. */
function movedAlong(part: Moved): boolean {
  return (
    part.turns === 0 &&
    !far(part.from.y, part.last.y) &&
    !far(part.from.width, part.last.width) &&
    !far(part.from.height, part.last.height)
  );
}

/** Whether a part under what grew was pushed once, by exactly that, without growing itself. */
function pushedInStep(part: Moved, pushed: number): boolean {
  return (
    part.steps <= 1 &&
    !far(part.from.x, part.last.x) &&
    !far(part.from.width, part.last.width) &&
    !far(part.from.height, part.last.height) &&
    !far(part.last.y - part.from.y, pushed)
  );
}

/**
 * Whether a part grew in place by exactly the rows that came into it (the Details around a row that
 * a change brings): a part holds what comes into it, and nothing else of it changed.
 */
function grewByWhatCameIn(part: Moved, appeared: Movement["appeared"]): boolean {
  const inside = appeared.filter((each) => holds(part.last, each.last));
  const cameIn = inside.reduce((sum, each) => sum + each.last.height, 0);
  return (
    inside.length > 0 &&
    part.turns === 0 &&
    !far(part.from.x, part.last.x) &&
    !far(part.from.y, part.last.y) &&
    !far(part.from.width, part.last.width) &&
    !far(part.last.height - part.from.height, cameIn)
  );
}

/**
 * The parts that moved in view, and whether any moved out of view only. What stands outside the
 * window at the start and at the end is not in view: the browser keeps what is in view still when
 * something above it grows, by its scroll (UX §6.4).
 */
function inViewOf(movement: Movement): {
  readonly seen: readonly Moved[];
  readonly anchored: boolean;
} {
  const { view } = movement;
  const inView = (box: Box): boolean =>
    view === undefined || (box.y < view && box.y + box.height > 0);
  const seen = movement.moved.filter((part) => inView(part.from) || inView(part.last));
  return { seen, anchored: seen.length < movement.moved.length };
}

/** What broke the rule of a still screen; all empty when it held. */
export interface Wrong {
  readonly moved: readonly Moved[];
  readonly shifts: readonly Shift[];
  readonly vanished: readonly string[];
  readonly greyShapes: boolean;
  readonly scrolled: readonly Scrolled[];
}

/**
 * What broke the rule of a still screen (UX §6.4): a part that moved by more than half a pixel unless
 * it grew in place (`grows`) or was pushed once by exactly what grew above it (`follows`); a layout
 * shift above nothing outside those parts, before or after an input; a part that went away; grey
 * shapes; a scroll that the action is not meant to make.
 */
export function wrongIn(movement: Movement, except: StillExcept): Wrong {
  const grows = except.grows ?? [];
  const follows = except.follows ?? [];
  const growths: Growth[] = [
    ...movement.moved
      .filter((part) => grows.includes(part.name))
      .map((part) => ({
        name: part.name,
        box: part.from,
        by: part.last.height - part.from.height,
        cameIn: false,
      })),
    ...movement.appeared.map((part) => ({
      name: part.name,
      box: part.last,
      by: part.last.height,
      cameIn: true,
    })),
  ];
  const moves = except.moves ?? [];
  const scrolls = except.scrolls ?? [];
  const may = new Set([...grows, ...follows, ...moves]);
  const theirs = (shift: Shift): boolean =>
    shift.sources.length > 0 &&
    shift.sources.every((source) => source.within.some((name) => may.has(name)));
  const { seen, anchored } = inViewOf(movement);
  const moved = seen.filter((part) => {
    const pushed = pushOf(part, growths);
    if (moves.includes(part.name)) {
      return !movedAlong(part);
    }
    if (grows.includes(part.name)) {
      return !grewInPlace(part, follows.includes(part.name) ? pushed : 0);
    }
    if (grewByWhatCameIn(part, movement.appeared)) {
      return false;
    }
    return !follows.includes(part.name) || !pushedInStep(part, pushed);
  });
  // A scroll that kept what is in view still while something above it changed is the browser's
  // own anchoring: allowed. Any other change of the scroll is wrong.
  const keptStill = anchored && moved.length === 0;
  return {
    moved,
    shifts: movement.shifts.filter((shift) => shift.value > 0 && !theirs(shift)),
    vanished: movement.vanished,
    greyShapes: movement.greyShapes,
    scrolled: keptStill ? [] : movement.scrolled.filter((each) => !scrolls.includes(each.part)),
  };
}

/** The answer of `wrongIn` when the screen stood still. */
export const nothingWrong: Wrong = {
  moved: [],
  shifts: [],
  vanished: [],
  greyShapes: false,
  scrolled: [],
};
