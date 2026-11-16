import type { Locator, Page } from "@playwright/test";

import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { chart, openTimeline } from "./support/timeline.ts";
import { openWarnings, prepareTimelinePeople } from "./support/timeline-people.ts";
import { viewports } from "../../tooling/viewports.ts";

// The room of the timeline on a computer (UX §4.7, §6.5): the box of the chart takes the whole height its column has,
// down to where the side ends, and the list of plan warnings stays whole inside the window, scrolling in its own box
// when it holds more than its share of the side, in a window of the usual height and in a low one.

test.use({ preparation: { run: prepareTimelinePeople } });

/** A window as low as a laptop with the browser's bars and a docked tool: the main column scrolls again. */
const LOW_WINDOW = { width: viewports.computer.width, height: 400 } as const;

/** A window higher than the rows of the chart and the side together. */
const HIGH_WINDOW = { width: viewports.computer.width, height: 1600 } as const;

/** How far one turn of the wheel goes: more than the main column holds. */
const WHEEL = 4000;

/** Where a box starts and ends on the screen, how much it holds and how high its window is. */
function placeOf(
  box: Locator,
): Promise<{ top: number; bottom: number; holds: number; shows: number; scrolls: string }> {
  return box.evaluate((node) => {
    const place = node.getBoundingClientRect();
    return {
      top: place.top,
      bottom: place.bottom,
      holds: node.scrollHeight,
      shows: node.clientHeight,
      scrolls: getComputedStyle(node).overflowY,
    };
  });
}

/** The box of the chart and the list of plan warnings, opened. */
async function openChartAndWarnings(page: Page): Promise<{ box: Locator; warnings: Locator }> {
  await signIn(page, ada);
  await openTimeline(page);
  const warnings = await openWarnings(page);
  return { box: page.locator("[data-part='timeline-scroll']"), warnings };
}

/** Turns the wheel over the gutter of the main column, beside its boxes, which scrolls the column itself. */
async function scrollTheMainColumn(page: Page): Promise<void> {
  const column = await page.locator("[data-part~='app-layout.main']").boundingBox();
  await page.mouse.move((column?.x ?? 0) + 2, (column?.y ?? 0) + (column?.height ?? 0) / 2);
  await page.mouse.wheel(0, WHEEL);
}

/** The list of warnings is whole inside the window, and a list that holds more than it shows scrolls in its own box. */
async function expectWarningsInside(page: Page, warnings: Locator): Promise<void> {
  const place = await placeOf(warnings);
  expect(place.top).toBeGreaterThanOrEqual(0);
  expect(place.bottom).toBeLessThanOrEqual(page.viewportSize()?.height ?? 0);
  expect(place.scrolls).toBe("auto");
  expect(place.holds).toBeGreaterThan(place.shows);
}

test("ux_06 the chart takes the whole height of its column, also below its last row", async ({
  page,
}) => {
  // A window higher than the rows of the chart: the box still ends where the side beside it ends.
  await page.setViewportSize(HIGH_WINDOW);
  const { box } = await openChartAndWarnings(page);
  const body = await placeOf(page.locator("[data-part='timeline-body']"));
  expect((await placeOf(chart(page))).bottom).toBeLessThan(body.bottom);
  await expect
    .poll(async () => Math.round((await placeOf(box)).bottom))
    .toBe(Math.round(body.bottom));
});

test("ux_06 the warnings stay whole in the window and scroll in their own box", async ({
  page,
}) => {
  const { warnings } = await openChartAndWarnings(page);
  await expectWarningsInside(page, warnings);
});

test("ux_06 in a low window the warnings stay whole in the window once the main column is scrolled", async ({
  page,
}) => {
  await page.setViewportSize(LOW_WINDOW);
  const { warnings } = await openChartAndWarnings(page);
  await scrollTheMainColumn(page);
  await expect(warnings).toBeInViewport({ ratio: 1 });
  await expectWarningsInside(page, warnings);
});
