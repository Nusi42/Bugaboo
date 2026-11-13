import { expect, type Locator, type Page, type Request } from "@playwright/test";

import {
  type Box,
  DONE_MARK,
  type Frames,
  recordFrames,
  recordShifts,
  SHIFTS_DONE,
  type Shift,
  twoFrames,
} from "./movement-record.ts";

export type { Box, Shift } from "./movement-record.ts";

/**
 * The measure of what moves on the screen while a person acts (UX §6, *Nothing moves under the cursor*;
 * architecture §11). It is the one way a browser test finds out what moved, flashed or emptied: no spec
 * has its own copy. In every frame the browser draws while the action runs it reads:
 *
 * - the browser's own record of layout shifts (`layout-shift`, every entry, also those right after an
 *   input, which the browser leaves out of its own score; WebKit keeps no such record, so there the
 *   boxes below are what holds);
 * - the box of each named landmark, and the scroll of the page and of every part that scrolls;
 * - whether grey shapes stood in for content, and whether a landmark that was there went away.
 */

/** A part of the screen that must stand still: its name in a finding, and the CSS selector of its first element. */
export interface Landmark {
  readonly name: string;
  readonly selector: string;
}

/** A landmark that stood somewhere else, or had another size, in a later frame than in the first. */
export interface Moved {
  readonly name: string;
  readonly from: Box;
  readonly to: Box;
  /** The frame it was first seen so, counted from the start of the recording. */
  readonly frame: number;
  /** How many times it stood somewhere else than in the frame before: a part that grows once has one step, one that jumps back and forth more. */
  readonly steps: number;
  /** Where it stood in the last frame. */
  readonly last: Box;
}

/** What moved, flashed or emptied while the action ran. */
export interface Movement {
  readonly moved: readonly Moved[];
  readonly shifts: readonly Shift[];
  /** The landmarks that were there at the start and gone in some frame after it. */
  readonly vanished: readonly string[];
  /** Whether grey shapes stood on the screen in any frame. */
  readonly greyShapes: boolean;
  /** The scroll of the page and its scrolling parts in the first frame and the first one that differed (none when nothing scrolled). */
  readonly scrolled: readonly [string, string] | undefined;
  readonly frames: number;
}

/** A difference of this many pixels or less is the rounding of a box, not a move. */
const LEAST_MOVE_PX = 1;

function differs(first: Box, later: Box): boolean {
  return (
    Math.abs(first.x - later.x) > LEAST_MOVE_PX ||
    Math.abs(first.y - later.y) > LEAST_MOVE_PX ||
    Math.abs(first.width - later.width) > LEAST_MOVE_PX ||
    Math.abs(first.height - later.height) > LEAST_MOVE_PX
  );
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
  const steps = shown.filter((box, index) => index > 0 && differs(shown[index - 1] ?? box, box));
  return { name, from: first, to, frame, steps: steps.length, last: shown.at(-1) ?? to };
}

/** What the frames say about the landmarks. */
function judged(landmarks: readonly Landmark[], frames: Frames) {
  const moved: Moved[] = [];
  const vanished: string[] = [];
  for (const [index, landmark] of landmarks.entries()) {
    const column = frames.boxes.map((frame) => frame[index] ?? null);
    if (column[0] !== null && column.includes(null)) {
      vanished.push(landmark.name);
    }
    const found = movedOf(landmark.name, column);
    if (found !== undefined) {
      moved.push(found);
    }
  }
  return { moved, vanished };
}

/** The path of the requests the measure waits for: those of the API. */
const API_PATH = "/api/";

/**
 * Counts the requests to the API that are on their way, from now on. `settled` waits until none is
 * left and none starts in the two frames after: what the action made the page read again is on the
 * screen then.
 */
function readsOnTheirWay(page: Page): { readonly settled: () => Promise<void> } {
  let open = 0;
  const started = (request: Readonly<Request>): void => {
    if (request.url().includes(API_PATH)) {
      open += 1;
    }
  };
  const ended = (request: Readonly<Request>): void => {
    if (request.url().includes(API_PATH)) {
      open -= 1;
    }
  };
  page.on("request", started);
  page.on("requestfinished", ended);
  page.on("requestfailed", ended);
  const settled = async (): Promise<void> => {
    await expect.poll(() => open).toBeLessThanOrEqual(0);
    await twoFrames(page);
    if (open > 0) {
      await settled();
      return;
    }
    page.off("request", started);
    page.off("requestfinished", ended);
    page.off("requestfailed", ended);
  };
  return { settled };
}

/**
 * What moved, flashed or emptied in every frame while `act` ran, and until every read of the API it
 * started has answered and been drawn. `act` ends when what the person waits for is on the screen
 * (the test waits for that inside it, as it would without the measure).
 */
export async function movementWhile(
  page: Page,
  landmarks: readonly Landmark[],
  act: () => Promise<void>,
): Promise<Movement> {
  const shifting = recordShifts(page);
  const recording = recordFrames(
    page,
    landmarks.map((landmark) => landmark.selector),
  );
  const reads = readsOnTheirWay(page);
  await twoFrames(page);
  await act();
  await reads.settled();
  await page.evaluate(
    (marks: readonly string[]) => {
      for (const mark of marks) {
        document.documentElement.setAttribute(mark, "");
      }
    },
    [DONE_MARK, SHIFTS_DONE],
  );
  const frames = await recording;
  const shifts = await shifting;
  const changed = frames.scrolls.find((each) => each !== frames.scrolls[0]);
  return {
    ...judged(landmarks, frames),
    shifts,
    greyShapes: frames.grey.includes(true),
    scrolled: changed === undefined ? undefined : [frames.scrolls[0] ?? "", changed],
    frames: frames.boxes.length,
  };
}

/** The landmark of the first element that is this part. */
export function partLandmark(name: string, part: string): Landmark {
  return { name, selector: `[data-part="${part}"]` };
}

/** The attribute that names the element of a landmark found by a locator. */
const LANDMARK = "data-landmark";

/**
 * The landmark of the element the locator finds now, marked with its name. An element that is drawn
 * afresh loses the mark, so it counts as gone: a part that is replaced instead of changed in place
 * is what the measure is there to find.
 */
export async function landmarkOf(found: Locator, name: string): Promise<Landmark> {
  await found.evaluate(
    (element, [attribute, value]) => {
      element.setAttribute(attribute, value);
    },
    [LANDMARK, name] as const,
  );
  return { name, selector: `[${LANDMARK}="${name}"]` };
}

/**
 * Fails unless the screen stood still (UX §6): no landmark moved, apart from those in `follows`, which
 * stand below what changed and may move once, in one step (a part that grows pushes what is under it
 * once, never back and forth); nothing went away, no grey shapes showed, and nothing scrolled.
 */
export function expectStill(movement: Movement, follows: readonly string[] = []): void {
  const wrong = movement.moved.filter((each) => !follows.includes(each.name) || each.steps > 1);
  expect(
    {
      moved: wrong,
      vanished: movement.vanished,
      greyShapes: movement.greyShapes,
      scrolled: movement.scrolled,
    },
    `what moved, went away, flashed or scrolled (shifts: ${JSON.stringify(movement.shifts)})`,
  ).toEqual({ moved: [], vanished: [], greyShapes: false, scrolled: undefined });
}
