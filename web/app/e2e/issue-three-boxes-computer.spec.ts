import type { Page } from "@playwright/test";

import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareManyWeb } from "./support/issues.ts";
import { issueAddress } from "../src/shared/paths.ts";

// On a computer an open issue in the focus view scrolls in three boxes of its own (UX §6.5): the slim list beside it,
// its sections and its Details. The wheel over one scrolls that one only; the others and the page stay where they are.

/** More issues than the slim list shows at once. */
const MORE_ISSUES = 30;

test.use({ preparation: { run: (server) => prepareManyWeb(server, MORE_ISSUES) } });

/** A window just higher than a low one (where the main column scrolls again), so each of the three holds more than it shows. */
const WINDOW = { width: 1280, height: 700 } as const;

/** How far one turn of the wheel scrolls a box: less than any of the boxes here holds. */
const WHEEL_STEP = 100;

/** Turns the wheel once over the middle of a box of the focus view, by its part. */
async function wheelOver(page: Page, part: string): Promise<void> {
  const box = await page.locator(`[data-part~='${part}']`).boundingBox();
  await page.mouse.move(
    (box?.x ?? 0) + (box?.width ?? 0) / 2,
    (box?.y ?? 0) + (box?.height ?? 0) / 2,
  );
  await page.mouse.wheel(0, WHEEL_STEP);
}

/** Where the slim list, the sections and the Details stand in their boxes, and the page itself. */
function scrollsOf(page: Page): Promise<number[]> {
  return page.evaluate(() =>
    [
      ...["focus-view.list", "issue-main", "issue-details"].map(
        (part) => document.querySelector(`[data-part~='${part}']`)?.scrollTop ?? -1,
      ),
      document.scrollingElement?.scrollTop ?? 0,
    ].map((place) => Math.round(place)),
  );
}

test("ux_06 the slim list, the sections and the Details each scroll on their own under the wheel", async ({
  page,
}) => {
  await page.setViewportSize(WINDOW);
  await signIn(page, ada);
  await page.goto(issueAddress("WEB-1"));
  await expect(page.locator("[data-part~='focus-view.list'] a").first()).toBeVisible();
  await expect(page.locator("[data-part='history.entry']").first()).toBeVisible();
  expect(await scrollsOf(page)).toEqual([0, 0, 0, 0]);
  // Each box under the wheel scrolls, and only it: the others and the page stay where they were.
  await wheelOver(page, "focus-view.list");
  await expect.poll(() => scrollsOf(page)).toEqual([WHEEL_STEP, 0, 0, 0]);
  await wheelOver(page, "issue-main");
  await expect.poll(() => scrollsOf(page)).toEqual([WHEEL_STEP, WHEEL_STEP, 0, 0]);
  await wheelOver(page, "issue-details");
  await expect.poll(() => scrollsOf(page)).toEqual([WHEEL_STEP, WHEEL_STEP, WHEEL_STEP, 0]);
});
