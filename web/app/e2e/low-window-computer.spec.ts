import type { Locator, Page } from "@playwright/test";

import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";
import { tickIssue } from "./support/list-look.ts";
import { openTimeline } from "./support/timeline.ts";
import { issueAddress, projectIssuesAddress } from "../src/shared/paths.ts";

// A low window on a computer (UX §6.5): the main column scrolls again, so what stands at the end of a pane is in reach
// with the wheel, as a person scrolls: the last row of a list, the bar of ticked issues, the end of the Details and
// *Not scheduled yet*.

test.use({ preparation: { run: prepareWeb } });

/** A window as low as a laptop with the browser's bars and a docked tool. */
const LOW_WINDOW = { width: 1280, height: 400 } as const;

/** How far one turn of the wheel goes: more than any pane here holds. */
const WHEEL = 4000;

/** Points at `box` where it shows on the screen, not under something laid over it (the bar of ticked issues). */
async function pointAt(page: Page, box: Locator): Promise<void> {
  const spot = await box.evaluate((node) => {
    const place = node.getBoundingClientRect();
    const x = place.left + 8;
    for (let y = Math.max(place.top, 0) + 4; y < Math.min(place.bottom, innerHeight); y += 4) {
      if (node.contains(document.elementFromPoint(x, y))) {
        return { x, y };
      }
    }
    return { x, y: place.top };
  });
  await page.mouse.move(spot.x, spot.y);
}

/** How far a box can still scroll down. */
function leftToScroll(box: Locator): Promise<number> {
  return box.evaluate((node) => node.scrollHeight - node.clientHeight - node.scrollTop);
}

/**
 * Turns the wheel down as a person does: over the gutter of the main column until the column is at its end, then over
 * `box` where it shows until `target` is whole in the window.
 */
async function wheelDown(page: Page, box: Locator, target: Locator): Promise<void> {
  const main = page.locator("[data-part~='app-layout.main']");
  // The gutter of the main column, beside its boxes, scrolls the column itself.
  const column = await main.boundingBox();
  await page.mouse.move((column?.x ?? 0) + 2, (column?.y ?? 0) + (column?.height ?? 0) / 2);
  await expect(async () => {
    await page.mouse.wheel(0, WHEEL);
    expect(await leftToScroll(main)).toBeLessThanOrEqual(1);
  }).toPass();
  await expect(async () => {
    await pointAt(page, box);
    await page.mouse.wheel(0, WHEEL);
    await expect(target).toBeInViewport({ ratio: 1, timeout: 1000 });
  }).toPass();
}

test.beforeEach(async ({ page }) => {
  await page.setViewportSize(LOW_WINDOW);
  await signIn(page, ada);
});

test("ux_06 in a low window the last row of a list and the bar of ticked issues are in reach", async ({
  page,
}) => {
  await page.goto(projectIssuesAddress("WEB"));
  const rows = page.locator("[data-part~='list-scroll']");
  await expect(rows).toBeVisible();
  await tickIssue(page, "WEB-1");
  await wheelDown(page, rows, rows.getByRole("row").last());
  await expect(page.locator("[data-part='selected-bar']")).toBeInViewport({ ratio: 1 });
});

test("ux_06 in a low window the last control of the Details is in reach", async ({ page }) => {
  await page.goto(issueAddress("WEB-1"));
  const details = page.locator(
    "[data-part='issue-body'][data-variant~='split'] [data-part='issue-details']",
  );
  await expect(details).toBeVisible();
  await wheelDown(page, details, details.locator("button, select, input, a[href]").last());
});

test("ux_06 in a low window Not scheduled yet is in reach", async ({ page }) => {
  await openTimeline(page);
  const tray = page.locator("[data-part='timeline-tray']");
  await wheelDown(page, tray, tray);
});
