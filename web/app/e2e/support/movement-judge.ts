import type { Box, Frames, Shift } from "./movement-record.ts";

// What the measure of movement (`movement.ts`) makes of the frames it recorded, and the rule it holds
// them to (UX §6.4). Pure, so the rule itself is tested without a browser (`movement-judge.test.ts`).

/** A landmark that stood somewhere else, or had another size, in a later frame than in the first. */
export interface Moved {
  readonly name: string;
  readonly from: Box;
  readonly to: Box;
  /** The frame it was first seen so, counted from the start of the recording. */
  readonly frame: number;
  /** How many times it stood somewhere else than in the frame before: one that is pushed once has one step, one that jumps back and forth more. */
  readonly steps: number;
  /** Where it stood in the last frame. */
  readonly last: Box;
  /** How many times its height turned from growing to shrinking or back, frame by frame. */
  readonly turns: number;
}

/** A landmark that was not there in the first frame and is there in the last (a row that came in). */
export interface Appeared {
  readonly name: string;
  readonly last: Box;
}

/** The page or a part that scrolled, and its scroll from the first frame to the first that differed. */
export interface Scrolled {
  readonly part: string;
  readonly from: string;
  readonly to: string;
}

/** What moved, flashed or emptied while the action ran. */
export interface Movement {
  readonly moved: readonly Moved[];
  readonly appeared: readonly Appeared[];
  readonly shifts: readonly Shift[];
  /** The landmarks that were there at the start and gone in some frame after it. */
  readonly vanished: readonly string[];
  /** Whether grey shapes stood on the screen in any frame. */
  readonly greyShapes: boolean;
  /** What scrolled: the page or each part whose scroll was not what it was in the first frame. */
  readonly scrolled: readonly Scrolled[];
  readonly frames: number;
}

/**
 * A difference of more than half a pixel is a move (UX §6.4: not even 1 px). Less is the subpixel
 * layout of the browser, which nobody sees.
 */
const LEAST_MOVE_PX = 0.5;

/** Whether two values of a box are more than half a pixel apart. */
function far(one: number, other: number): boolean {
  return Math.abs(one - other) > LEAST_MOVE_PX;
}

/** Whether a box stands somewhere else, or has another width: its height growing is not that. */
function standsElsewhere(first: Box, later: Box): boolean {
  return far(first.x, later.x) || far(first.y, later.y) || far(first.width, later.width);
}

function differs(first: Box, later: Box): boolean {
  return standsElsewhere(first, later) || far(first.height, later.height);
}

/** How many times a height turned from growing to shrinking or back, frame by frame. */
function turnsOf(shown: readonly Box[]): number {
  const signs = shown
    .map((box, index) => box.height - (shown[index - 1] ?? box).height)
    .filter((change) => Math.abs(change) > LEAST_MOVE_PX)
    .map((change) => Math.sign(change));
  return signs.filter((sign, index) => index > 0 && sign !== signs[index - 1]).length;
}

/** The first frame in which this landmark stood elsewhere than in the first frame, if any. */
function movedOf(name: string, column: readonly (Box | null)[]): Moved | undefined {
  const [first] = column;
  if (first === null || first === undefined) {
    return undefined;
  }
  const frame = column.findIndex((box) => box !== null && differs(first, box));
  const to = column[frame] ?? null;
  if (to === null) {
    return undefined;
  }
  const shown = column.filter((box) => box !== null);
  const steps = shown.filter(
    (box, index) => index > 0 && standsElsewhere(shown[index - 1] ?? box, box),
  ).length;
  return { name, from: first, to, frame, steps, last: shown.at(-1) ?? to, turns: turnsOf(shown) };
}

/** What the frames say about the landmarks, named in the order of `names`, each with whether it can be found again. */
export function judged(names: readonly (readonly [string, boolean])[], frames: Frames) {
  const moved: Moved[] = [];
  const appeared: Appeared[] = [];
  const vanished: string[] = [];
  for (const [index, [name, findsAgain]] of names.entries()) {
    const column = frames.boxes.map((frame) => frame[index] ?? null);
    const [first] = column;
    const last = column.at(-1) ?? null;
    // A part that can be found again (a row of a list, by its key) and is gone with nothing in its
    // place only left the view (scrolled away, or no longer drawn by the list); drawn afresh, it went.
    const redrawn = frames.redrawn.some((frame) => frame[index] === true);
    if (first !== null && (redrawn || (!findsAgain && column.includes(null)))) {
      vanished.push(name);
    }
    if ((first ?? null) === null && last !== null) {
      appeared.push({ name, last });
    }
    const found = movedOf(name, column);
    if (found !== undefined) {
      moved.push(found);
    }
  }
  return { moved, appeared, vanished };
}

/** The scroll of each part in one frame, as the recording writes it (`page:0 list-scroll:40/0`). */
function scrollsOf(frame: string): ReadonlyMap<string, string> {
  return new Map(
    frame.split(" ").map((each) => {
      const at = each.lastIndexOf(":");
      return [each.slice(0, at), each.slice(at + 1)] as const;
    }),
  );
}

/** Every part whose scroll in some frame was not what it was in the first frame. */
export function scrolledOf(frames: readonly string[]): readonly Scrolled[] {
  const first = scrollsOf(frames[0] ?? "");
  const found = new Map<string, Scrolled>();
  for (const frame of frames) {
    const now = scrollsOf(frame);
    for (const part of new Set([...first.keys(), ...now.keys()])) {
      const [from, to] = [first.get(part) ?? "0/0", now.get(part) ?? "0/0"];
      if (from !== to && !found.has(part)) {
        found.set(part, { part, from, to });
      }
    }
  }
  return [...found.values()];
}

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
  const may = new Set([...grows, ...follows]);
  const theirs = (shift: Shift): boolean =>
    shift.sources.length > 0 &&
    shift.sources.every((source) => source.within.some((name) => may.has(name)));
  return {
    moved: movement.moved.filter((part) => {
      const pushed = pushOf(part, growths);
      if (grows.includes(part.name)) {
        return !grewInPlace(part, follows.includes(part.name) ? pushed : 0);
      }
      return !follows.includes(part.name) || !pushedInStep(part, pushed);
    }),
    shifts: movement.shifts.filter((shift) => shift.value > 0 && !theirs(shift)),
    vanished: movement.vanished,
    greyShapes: movement.greyShapes,
    scrolled: movement.scrolled.filter((each) => !(except.scrolls ?? []).includes(each.part)),
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
