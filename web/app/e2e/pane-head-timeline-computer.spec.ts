import type { Locator, Page } from "@playwright/test";

import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { expectStill, movementWhile, partLandmark } from "./support/movement.ts";
import {
  coveredSteps,
  focusInHead,
  foldsWith,
  headGap,
  keyThrough,
  MAIN_COLUMN,
  PANE_HEAD,
  wheelOver,
} from "./support/pane-head.ts";
import { inOrder } from "./support/sequence.ts";
import { chart, prepareLongTimeline } from "./support/timeline.ts";
import { projectIssuesAddress } from "../src/shared/paths.ts";

// The timeline in the list of a project (UX §6.5): its tools and Legend stand in the head of the pane, which folds to the
// toolbar as the chart or *Not scheduled yet* scrolls and opens again at their top; the scale of the chart stops under
// the folded head, and nothing else moves.

/** More rows with dates than the chart shows, and more without than *Not scheduled yet* shows. */
const DATED = 30;
const UNDATED = 20;

test.use({ preparation: { run: (server) => prepareLongTimeline(server, DATED, UNDATED) } });

/** A window as high as a laptop's, above the low window: the chart and its side fit the room the main column leaves. */
const LAPTOP = { width: 1280, height: 720 } as const;

/** The box of the chart. */
const CHART = "[data-part~='timeline-scroll']";

/** The scale at the top of the chart. */
const SCALE = "[data-part~='timeline-head']";

/** The box of *Not scheduled yet*. */
const TRAY = "[data-part~='timeline-tray']";

/** Opens the list of the project as its timeline and waits until it is read whole. */
async function openTimeline(page: Page): Promise<void> {
  await page.setViewportSize(LAPTOP);
  await signIn(page, ada);
  await page.goto(`${projectIssuesAddress("WEB")}?show=timeline`);
  await expect(chart(page)).toBeVisible();
  await page.waitForLoadState("networkidle");
}

/** Turns the wheel over `box` and judges every frame: only `box` scrolls, the head folds, nothing else moves. */
async function stillWhileScrolling(page: Page, box: Locator, scrolls: string): Promise<void> {
  const movement = await movementWhile(
    page,
    [
      partLandmark("the heading of the card", "card.heading"),
      partLandmark("the toolbar", "list-toolbar"),
      partLandmark("the chart", "timeline-scroll"),
      partLandmark("Not scheduled yet", "timeline-tray"),
    ],
    () => wheelOver(page, box),
  );
  expectStill(movement, { scrolls: [scrolls] });
}

test("ux_01 the head of the timeline folds as the chart scrolls and nothing else moves in any frame", async ({
  page,
}) => {
  await openTimeline(page);
  const box = page.locator(CHART);
  await stillWhileScrolling(page, box, "timeline-scroll");
  await foldsWith(page, box);
});

test("ux_01 the head of the timeline folds as Not scheduled yet scrolls and nothing else moves in any frame", async ({
  page,
}) => {
  await openTimeline(page);
  const box = page.locator(TRAY);
  await stillWhileScrolling(page, box, "timeline-tray");
  await foldsWith(page, box);
});

/** How far the top of the scale stands below the bottom of the head of the pane, once the chart is drawn. */
function scaleUnderHead(page: Page): Promise<number> {
  return page.evaluate(
    ([scale, head]) =>
      (document.querySelector(scale)?.getBoundingClientRect().top ?? 0) -
      (document.querySelector(head)?.getBoundingClientRect().bottom ?? 0),
    [SCALE, PANE_HEAD] as const,
  );
}

test("ux_06 the scale of the chart stands right under the head, whole or folded", async ({
  page,
}) => {
  await openTimeline(page);
  const box = page.locator(CHART);
  const scrollTo = (to: number) =>
    box.evaluate((node, top) => {
      node.scrollTop = top;
    }, to);
  // At most the edge of the box below the head, and the same while the head folds and once it is folded.
  const whole = await scaleUnderHead(page);
  expect(whole).toBeGreaterThanOrEqual(0);
  expect(whole).toBeLessThanOrEqual(1);
  await scrollTo(60);
  await expect.poll(() => scaleUnderHead(page)).toBeCloseTo(whole, 0);
  await scrollTo(100_000);
  await expect.poll(() => scaleUnderHead(page)).toBeCloseTo(whole, 0);
});

test("ux_06 the keyboard brings a bar out from under the folded head and the scale", async ({
  page,
}) => {
  await openTimeline(page);
  const box = page.locator(CHART);
  // From the last bar, with the box at its end and the head folded, back up through the chart.
  await box.locator("button, a[href], [tabindex='0']").last().focus();
  await box.evaluate((node) => {
    node.scrollTop = node.scrollHeight;
  });
  const steps = await keyThrough(page, "Shift+Tab", 16, CHART, SCALE);
  expect(steps.filter((step) => step.inside).length).toBeGreaterThan(8);
  expect(coveredSteps(steps), "the bars the head or the scale covered").toEqual([]);
});

test("ux_06 on paper the chart stands under the Legend as on the screen", async ({ page }) => {
  await openTimeline(page);
  const gap = () => headGap(page, "[data-part~='timeline-canvas']");
  const onScreen = await gap();
  await page.emulateMedia({ media: "print" });
  await expect.poll(gap).toBeCloseTo(onScreen, 0);
});

/** Whether the folded head is what the browser draws at its middle, at twelve places across the chart. */
function headOverChart(page: Page): Promise<boolean[]> {
  return page.evaluate(
    ([head, chartBox]) => {
      const folded = document.querySelector(head)?.getBoundingClientRect() ?? new DOMRect();
      const across = document.querySelector(chartBox)?.getBoundingClientRect() ?? new DOMRect();
      const middle = (folded.top + folded.bottom) / 2;
      return Array.from(
        { length: 12 },
        (_, step) =>
          document
            .elementFromPoint(across.left + ((step + 0.5) * across.width) / 12, middle)
            ?.closest(head) instanceof HTMLElement,
      );
    },
    [PANE_HEAD, CHART] as const,
  );
}

/** The index of the first bar below the middle of the window. */
function firstBelowMiddle(bars: readonly HTMLElement[]): number {
  return bars.findIndex((bar) => bar.getBoundingClientRect().top > innerHeight / 2);
}

/** With the chart at its end: whether its last row stands in the box, and the box in the window. */
function endsOfChart(page: Page): Promise<{ lastRowInBox: boolean; boxInWindow: boolean }> {
  return page.locator(CHART).evaluate((node) => {
    const last = [...node.querySelectorAll("[data-part~='timeline-row']")].at(-1);
    const place = node.getBoundingClientRect();
    const end = place.top + node.clientTop + node.clientHeight + 0.5;
    return {
      lastRowInBox: (last?.getBoundingClientRect().bottom ?? Infinity) <= end,
      boxInWindow: place.bottom <= innerHeight,
    };
  });
}

/** Scrolls the chart this far down and waits until it is drawn there. */
async function chartAt(page: Page, top: number): Promise<void> {
  const box = page.locator(CHART);
  await box.evaluate((node, to) => {
    node.scrollTop = to;
  }, top);
  await expect.poll(() => scaleUnderHead(page)).toBeLessThanOrEqual(1);
}

test("ux_06 the folded head stands over the chart, its scale, the names of the rows and the Today line", async ({
  page,
}) => {
  await openTimeline(page);
  await chartAt(page, 400);
  // Across the whole width of the chart, at the middle of the folded head, the head is what the browser draws.
  const covered = await headOverChart(page);
  expect(covered).toEqual(covered.map(() => true));
});

test("ux_06 a control of the head reached with the keyboard from the scrolled chart shows whole, over the scale", async ({
  page,
}) => {
  await openTimeline(page);
  await chartAt(page, 400);
  // From a bar in view, back up through the chart into the head, which stays whole while one of its controls has the
  // focus.
  const bars = page.locator(`${CHART} [data-part~='timeline-bar.press']`);
  const inView = await bars.evaluateAll(firstBelowMiddle);
  await bars.nth(inView).focus();
  const seen: Readonly<Awaited<ReturnType<typeof focusInHead>>>[] = [];
  await inOrder(
    Array.from({ length: 30 }, () => async () => {
      await page.keyboard.press("Shift+Tab");
      seen.push(await focusInHead(page));
    }),
  );
  const inHead = seen.filter((step) => step.inside);
  expect(inHead.length, "the controls of the head reached").toBeGreaterThan(5);
  expect(inHead).toEqual(inHead.map(() => ({ inside: true, onTop: true, scrolled: 0 })));
});

test("ux_06 at a laptop's height the last row and the end of the chart can be scrolled into view", async ({
  page,
}) => {
  await openTimeline(page);
  const box = page.locator(CHART);
  await box.evaluate((node) => {
    node.scrollTop = node.scrollHeight;
  });
  const ends = await endsOfChart(page);
  expect(ends).toEqual({ lastRowInBox: true, boxInWindow: true });
  // The main column holds no more than it shows: nothing of the page is out of reach below it.
  const main = page.locator(MAIN_COLUMN);
  expect(await main.evaluate((node) => node.scrollHeight - node.clientHeight)).toBeLessThanOrEqual(
    0,
  );
});
