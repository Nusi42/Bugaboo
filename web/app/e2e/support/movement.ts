import { expect, type Locator, type Page, type Request } from "@playwright/test";

import {
  judged,
  type Movement,
  nothingWrong,
  type Scrolled,
  scrolledOf,
  type StillExcept,
  wrongIn,
} from "./movement-judge.ts";
import {
  DONE_MARK,
  recordFrames,
  recordPinned,
  recordShifts,
  stampLandmarks,
  SHIFTS_DONE,
  twoFrames,
} from "./movement-record.ts";

export type { Box, Shift } from "./movement-record.ts";
export type { Movement, Moved, Scrolled, StillExcept, Wrong } from "./movement-judge.ts";
export { nothingWrong, wrongIn } from "./movement-judge.ts";

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
  /**
   * How to find the same thing drawn afresh, where it can be told apart from leaving the view (a row
   * of a list, by its key): gone with nothing in its place, it only left the view.
   */
  readonly again?: string;
}

/** The path of the requests the measure waits for: those of the API. */
const API_PATH = "/api/";

/**
 * Counts the requests to the API that are on their way, from now on. `settled` waits until none is
 * left and none starts in the two frames after: what the action made the page read again is on the
 * screen then.
 */
export function readsOnTheirWay(page: Page): { readonly settled: () => Promise<void> } {
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
    landmarks.map((landmark) => [landmark.selector, landmark.again ?? null] as const),
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
  const pinned = await recordPinned(
    page,
    landmarks.map((landmark) => landmark.selector),
  );
  return {
    ...judged(
      landmarks.map((landmark) => [landmark.name, landmark.again !== undefined] as const),
      { ...frames, pinned },
    ),
    shifts,
    greyShapes: frames.grey.includes(true),
    scrolled: await withBoxes(page, scrolledOf(frames.scrolls)),
    frames: frames.boxes.length,
    view: frames.view,
  };
}

/** Each part that scrolled, with where its box stands now (the page: the window). */
function withBoxes(page: Page, scrolled: readonly Scrolled[]): Promise<readonly Scrolled[]> {
  return page.evaluate(
    (parts) =>
      parts.map((each) => {
        const found =
          each.part === "page"
            ? null
            : document.querySelector(`[data-part="${each.part}"]`)?.getBoundingClientRect();
        const box =
          found === undefined
            ? undefined
            : found === null
              ? { x: 0, y: 0, width: innerWidth, height: innerHeight }
              : { x: found.x, y: found.y, width: found.width, height: found.height };
        return box === undefined ? each : { ...each, box };
      }),
    scrolled,
  );
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

/**
 * Fails unless the screen stood still (UX §6.4), by the rule of `wrongIn`: what the action changes
 * grows in place, what stands under it is pushed once by exactly that, and nothing else moves,
 * shifts, goes, flashes or scrolls.
 */
export function expectStill(movement: Movement, except: StillExcept = {}): void {
  expect(wrongIn(movement, except), "what moved, shifted, went away, flashed or scrolled").toEqual(
    nothingWrong,
  );
}

/** Fails when the landmark got smaller while the action ran (the bar over the foot of a list takes no room from it). */
export function expectNotSmaller(movement: Movement, name: string): void {
  const part = movement.moved.find((each) => each.name === name);
  expect(part?.last.height ?? 0, `${name} got smaller`).toBeGreaterThanOrEqual(
    (part?.from.height ?? 0) - 0.5,
  );
}
