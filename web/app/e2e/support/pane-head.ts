import type { Locator, Page } from "@playwright/test";

import { expect } from "./fixtures.ts";
import { layoutOf } from "./layout.ts";
import { inOrder } from "./sequence.ts";

// The head of a pane that folds as its boxes scroll (UX §6.5): how the browser tests scroll a box as a person does and
// what they read of the head meanwhile.

/** A low window, so the boxes of the pane are higher than their room. */
export const LOW_WINDOW = { width: 1280, height: 520 } as const;

/** How far a person scrolls in one step: less than the head can fold. */
const STEP = 30;

/** The head of a pane. */
export const PANE_HEAD = "[data-part='pane-head']";

/** The boxes of an issue open on a computer, side by side under its head. */
export const SPLIT_ISSUE = "[data-part='issue-body'][data-variant~='split']";

/** The box of a topic of the settings, under its head. */
export const SETTINGS_TOPIC = "[data-part~='settings-topic']";

/** The main column of the app. */
export const MAIN_COLUMN = "[data-part~='app-layout.main']";

/**
 * The box that scrolls a page of the settings: on a computer the topic under the head that folds over it, elsewhere the
 * main column (UX §6.5).
 */
export function settingsBoxOf(page: Page): string {
  return layoutOf(page) === "computer" ? SETTINGS_TOPIC : MAIN_COLUMN;
}

/** The lines of the head: its own, and those that deeper parts put into its slots, which are no boxes of their own. */
const HEAD_LINES =
  "[data-part='pane-head.content'] > :not([data-part='pane-head.slot']), [data-part='pane-head.slot'] > *";

/** The head of the pane on the screen. */
function headOf(page: Page): Locator {
  return page.locator(PANE_HEAD);
}

/** How high an element is drawn now. */
function heightOf(element: Locator): Promise<number> {
  return element.evaluate((node) => node.getBoundingClientRect().height);
}

/**
 * Scrolls a box as a person does, waits for the next frame, and says where the box then stands and how far its first
 * element moved on the screen.
 */
function scrollBy(box: Locator, by: number): Promise<{ scrolled: number; moved: number }> {
  return box.evaluate(async (node, step) => {
    const first = node.firstElementChild ?? node;
    const before = first.getBoundingClientRect().top;
    const start = node.scrollTop;
    node.scrollTop = start + step;
    await new Promise<void>((done) => {
      requestAnimationFrame(() => {
        requestAnimationFrame(() => {
          done();
        });
      });
    });
    return {
      scrolled: node.scrollTop - start,
      moved: before - first.getBoundingClientRect().top,
    };
  }, by);
}

/**
 * What the folded head shows: whether its first line stands whole inside it, whether any of the next line shows, and
 * how far the head itself is scrolled (never: it is cut, not scrolled).
 */
function foldedOf(page: Page): Promise<{ first: boolean; second: boolean; scrolled: number }> {
  return headOf(page).evaluate((node, selector) => {
    const box = node.getBoundingClientRect();
    const lines: DOMRect[] = [];
    for (const line of node.querySelectorAll(selector)) {
      const place = line.getBoundingClientRect();
      if (place.height > 0) {
        lines.push(place);
      }
    }
    const [first, second] = lines;
    return {
      first: first !== undefined && first.top >= box.top && first.bottom <= box.bottom,
      second: second !== undefined && second.top < box.bottom,
      scrolled: node.scrollTop,
    };
  }, HEAD_LINES);
}

/** The head folds as `box` scrolls, the box moves only by the scroll, and the head opens again at the top. */
export async function foldsWith(page: Page, box: Locator): Promise<void> {
  const head = headOf(page);
  await expect(head).toBeVisible();
  // Once the box holds more than it shows: before that, there is nothing to scroll.
  await expect
    .poll(() => box.evaluate((node) => node.scrollHeight - node.clientHeight))
    .toBeGreaterThan(STEP);
  const whole = await heightOf(head);
  const step = await scrollBy(box, STEP);
  expect(step).toEqual({ scrolled: STEP, moved: STEP });
  const folding = await heightOf(head);
  expect(folding).toBeLessThan(whole);
  // Far down, the head is its first line, whole, and nothing else of it; the scroll stays where the person left it.
  await box.evaluate((node) => {
    node.scrollTop = node.scrollHeight;
  });
  const end = await box.evaluate((node) => node.scrollTop);
  await expect.poll(() => heightOf(head)).toBeLessThan(folding);
  await expect.poll(() => foldedOf(page)).toEqual({ first: true, second: false, scrolled: 0 });
  expect(await box.evaluate((node) => node.scrollTop)).toBe(end);
  // Back at the top the head is whole again.
  await box.evaluate((node) => {
    node.scrollTop = 0;
  });
  await expect.poll(() => heightOf(head)).toBeCloseTo(whole, 0);
  expect(await box.evaluate((node) => node.scrollTop)).toBe(0);
}

/** How many turns of the wheel a person makes, each less than the head can fold. */
const TURNS = 8;

/** Turns the wheel over `box` a few small steps down, then as many up, as a person does. */
export async function wheelOver(page: Page, box: Locator): Promise<void> {
  // Near the end of the box where the window shows it: its start is under the head, and in a low window the main
  // column may hold its end below the window.
  const spot = await box.evaluate((node) => {
    const place = node.getBoundingClientRect();
    return { x: place.left + 8, y: Math.min(place.bottom, innerHeight) - 8 };
  });
  await page.mouse.move(spot.x, spot.y);
  // One turn after the other, each once the box has taken the one before, as a hand turns the wheel.
  const turns = (way: number, from: number) =>
    Array.from({ length: TURNS }, (_, turn) => async () => {
      await page.mouse.wheel(0, way);
      await expect.poll(() => box.evaluate((node) => node.scrollTop)).toBe(from + (turn + 1) * way);
    });
  await inOrder(turns(STEP, 0));
  await inOrder(turns(-STEP, TURNS * STEP));
}

/**
 * Where the focus stands after a press of a key: whether it is in the box, how its top stands to what covers it, and
 * whether what the browser draws at its middle is the cover or the head of the pane rather than the focus.
 */
export interface FocusStep {
  readonly inside: boolean;
  readonly top: number;
  readonly covered: number;
  readonly underCover: boolean;
}

/**
 * Presses `key` `times` times, as a person moves through a box with the keyboard, and says after each press where the
 * focus stands against the bottom of `cover` (the folded head, or the head of the columns under it).
 */
export async function keyThrough(
  page: Page,
  key: string,
  times: number,
  box: string,
  cover: string,
): Promise<FocusStep[]> {
  const steps: FocusStep[] = [];
  await inOrder(
    Array.from({ length: times }, () => async () => {
      await page.keyboard.press(key);
      steps.push(
        await page.evaluate(
          async ([inBox, over, head]) => {
            // What the press scrolled is drawn in the next frames.
            await new Promise<void>((done) => {
              requestAnimationFrame(() => {
                requestAnimationFrame(() => {
                  done();
                });
              });
            });
            const active = document.activeElement;
            const place = active?.getBoundingClientRect();
            const hit =
              place === undefined
                ? null
                : document.elementFromPoint(
                    place.left + place.width / 2,
                    place.top + place.height / 2,
                  );
            return {
              inside: active?.closest(inBox) !== null && active !== null,
              top: place?.top ?? 0,
              covered: document.querySelector(over)?.getBoundingClientRect().bottom ?? 0,
              underCover:
                hit !== null &&
                active?.contains(hit) !== true &&
                hit.closest(`${over}, ${head}`) !== null,
            };
          },
          [box, cover, PANE_HEAD] as const,
        ),
      );
    }),
  );
  return steps;
}

/** The steps of `keyThrough` inside the box where the focus stood under what covers it, or under anything else. */
export function coveredSteps(steps: readonly FocusStep[]): readonly FocusStep[] {
  return steps.filter((step) => step.inside && (step.top < step.covered - 0.5 || step.underCover));
}

/**
 * Whether the focused element stands whole inside the head of the pane and is what the browser draws on top at its
 * middle (nothing of the page covers it), and how far the head itself is scrolled.
 */
export function focusInHead(
  page: Page,
): Promise<{ inside: boolean; onTop: boolean; scrolled: number }> {
  return headOf(page).evaluate((head) => {
    const box = head.getBoundingClientRect();
    const focused = document.activeElement;
    const place = focused?.getBoundingClientRect();
    const hit =
      place === undefined
        ? null
        : document.elementFromPoint(place.left + place.width / 2, place.top + place.height / 2);
    return {
      inside: place !== undefined && place.top >= box.top && place.bottom <= box.bottom,
      onTop: hit !== null && focused !== null && focused.contains(hit),
      scrolled: head.scrollTop,
    };
  });
}

/** How far the first row of a list stands below the head of its columns. */
export function firstRowGap(page: Page): Promise<number> {
  return page.evaluate(() => {
    const head = document.querySelector("[data-part='list-head']")?.getBoundingClientRect();
    let top = Infinity;
    for (const row of document.querySelectorAll("[data-part='list-scroll'] [role='row']")) {
      top = Math.min(top, row.getBoundingClientRect().top);
    }
    return top - (head?.bottom ?? 0);
  });
}

/** The background of the head of the pane and of what holds the pane: the nearest box around it with a background. */
export function coverColors(page: Page): Promise<{ head: string; holder: string }> {
  return headOf(page).evaluate((head) => {
    const clear = "rgba(0, 0, 0, 0)";
    let holder = head.parentElement?.parentElement ?? null;
    while (holder !== null && getComputedStyle(holder).backgroundColor === clear) {
      holder = holder.parentElement;
    }
    return {
      head: getComputedStyle(head).backgroundColor,
      holder: holder === null ? clear : getComputedStyle(holder).backgroundColor,
    };
  });
}

/** How far the first part of `box` stands below the last line of the head of the pane. */
export function headGap(page: Page, box: string): Promise<number> {
  return headOf(page).evaluate(
    (head, [selector, lines]) => {
      let end = -Infinity;
      for (const line of head.querySelectorAll(lines)) {
        const place = line.getBoundingClientRect();
        end = place.height > 0 ? Math.max(end, place.bottom) : end;
      }
      const first = document.querySelector(selector)?.firstElementChild;
      return (first?.getBoundingClientRect().top ?? 0) - end;
    },
    [box, HEAD_LINES] as const,
  );
}
