import type { Locator, Page } from "@playwright/test";

import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { expectStill, movementWhile, partLandmark } from "./support/movement.ts";
import {
  coveredSteps,
  foldsWith,
  headGap,
  keyThrough,
  PANE_HEAD,
  wheelOver,
} from "./support/pane-head.ts";
import { chart, prepareLongTimeline } from "./support/timeline.ts";
import { projectIssuesAddress } from "../src/shared/paths.ts";

// The timeline in the list of a project (UX §6.5): its tools and Legend stand in the head of the pane, which folds to the
// toolbar as the chart or *Not scheduled yet* scrolls and opens again at their top; the scale of the chart stops under
// the folded head, and nothing else moves.

/** More rows with dates than the chart shows, and more without than *Not scheduled yet* shows. */
const DATED = 30;
const UNDATED = 20;

test.use({ preparation: { run: (server) => prepareLongTimeline(server, DATED, UNDATED) } });

/** A window as high as a laptop's, above the low window: the main column fits it and does not scroll. */
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
