import type { Box, Frames, Shift } from "./movement-record.ts";

// What the frames the measure of movement recorded say (`movement.ts`): which landmarks moved, came
// in, went away or scrolled. The rule they are held to is `movement-judge.ts`.

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
  /** The height of the window, when it was recorded: what stands outside it is not in view. */
  readonly view?: number;
}

/**
 * A difference of more than half a pixel is a move (UX §6.4: not even 1 px). Less is the subpixel
 * layout of the browser, which nobody sees.
 */
export const LEAST_MOVE_PX = 0.5;

/** Whether two values of a box are more than half a pixel apart. */
export function far(one: number, other: number): boolean {
  return Math.abs(one - other) > LEAST_MOVE_PX;
}

/** Whether a box stands somewhere else, or has another width: its height growing is not that. */
function standsElsewhere(first: Box, later: Box): boolean {
  return far(first.x, later.x) || far(first.y, later.y) || far(first.width, later.width);
}

/** Whether a box stands somewhere else or has another size. */
export function differs(first: Box, later: Box): boolean {
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
