import { expect, type Locator, type Page, type Request } from "@playwright/test";

import {
  type Box,
  DONE_MARK,
  type Frames,
  recordFrames,
  recordShifts,
  stampLandmarks,
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
  /** How many times it stood somewhere else than in the frame before: one that is pushed once has one step, one that jumps back and forth more. */
  readonly steps: number;
  /** How many times its height changed: a part that grows downwards once has one. */
  readonly growths: number;
  /** Where it stood in the last frame. */
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
  readonly shifts: readonly Shift[];
  /** The landmarks that were there at the start and gone in some frame after it. */
  readonly vanished: readonly string[];
  /** Whether grey shapes stood on the screen in any frame. */
  readonly greyShapes: boolean;
  /** What scrolled: the page or each part whose scroll was not what it was in the first frame, with its scrolls (`list-scroll: 0/0 -> 40/0`). */
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
  const changes = (changed: (before: Box, after: Box) => boolean): number =>
    shown.filter((box, index) => index > 0 && changed(shown[index - 1] ?? box, box)).length;
  return {
    name,
    from: first,
    to,
    frame,
    steps: changes(standsElsewhere),
    growths: changes((before, after) => far(before.height, after.height)),
    last: shown.at(-1) ?? to,
  };
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
function scrolledOf(frames: readonly string[]): readonly Scrolled[] {
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

/** The path of the requests the measure waits for: those of the API. */
const API_PATH = "/api/";

/**
 * Counts the requests to the API that are on their way, from now on. `settled` waits until none is
 * left and none starts in the two frames after: what the action made the page read again is on the
 * screen then.
 */
function readsOnTheirWay(page: Page): { readonly settled: () => Promise<void> } {
  // Only requests seen from now on count: one that began before cannot end the wait early.
  const open = new Set<Readonly<Request>>();
  const started = (request: Readonly<Request>): void => {
    if (request.url().includes(API_PATH)) {
      open.add(request);
    }
  };
  const ended = (request: Readonly<Request>): void => {
    open.delete(request);
  };
  page.on("request", started);
  page.on("requestfinished", ended);
  page.on("requestfailed", ended);
  const settled = async (): Promise<void> => {
    await expect.poll(() => open.size).toBe(0);
    await twoFrames(page);
    if (open.size > 0) {
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
  await stampLandmarks(
    page,
    landmarks.map((landmark) => [landmark.name, landmark.selector] as const),
  );
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
  return {
    ...judged(landmarks, frames),
    shifts,
    greyShapes: frames.grey.includes(true),
    scrolled: scrolledOf(frames.scrolls),
    frames: frames.boxes.length,
  };
}

/** The landmark of the first element that is this part. */
export function partLandmark(name: string, part: string): Landmark {
  return { name, selector: `[data-part="${part}"]` };
}

/** The attribute that names the element of a landmark found by a locator or on a whole screen. */
export const LANDMARK = "data-landmark";

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

/** What may change while the screen otherwise stands still. */
export interface StillExcept {
  /**
   * The landmarks under what changed, and what changed itself: each may be pushed once and grow
   * downwards once (a part that grows pushes what is under it once, never back and forth), and
   * layout shifts inside them are theirs.
   */
  readonly follows?: readonly string[];
  /** The parts whose scroll the action is meant to change (`window.body` when a form starts again). */
  readonly scrolls?: readonly string[];
}

/**
 * Fails unless the screen stood still (UX §6.4): no landmark moved by more than half a pixel, apart from
 * those in `follows`; no layout shift above nothing outside them, before or after an input; nothing
 * went away, no grey shapes showed, and nothing scrolled but what `scrolls` names.
 */
export function expectStill(movement: Movement, except: StillExcept = {}): void {
  const follows = except.follows ?? [];
  const inFollows = (shift: Shift): boolean =>
    shift.sources.length > 0 &&
    shift.sources.every((source) => source.within.some((name) => follows.includes(name)));
  expect(
    {
      moved: movement.moved.filter(
        (each) => !follows.includes(each.name) || each.steps > 1 || each.growths > 1,
      ),
      shifts: movement.shifts.filter((shift) => shift.value > 0 && !inFollows(shift)),
      vanished: movement.vanished,
      greyShapes: movement.greyShapes,
      scrolled: movement.scrolled.filter((each) => !(except.scrolls ?? []).includes(each.part)),
    },
    "what moved, shifted, went away, flashed or scrolled",
  ).toEqual({ moved: [], shifts: [], vanished: [], greyShapes: false, scrolled: [] });
}
