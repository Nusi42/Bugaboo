import type { Locator, Page } from "@playwright/test";

import { t } from "@bugaboo/messages";

import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareManyWeb, prepareWeb } from "./support/issues.ts";
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

/**
 * How high a pane beside the list is, how much its box of `inner` holds, and how high the line it stands in is: as high
 * as the list beside it.
 */
function besideList(
  side: Locator,
  inner: string,
): Promise<{ high: number; holds: number; line: number }> {
  return side.evaluate((node, part) => {
    const scrolls = node.querySelector(`[data-part~='${part}']`) ?? node;
    const line = node.parentElement?.getBoundingClientRect().height ?? 0;
    return { high: node.getBoundingClientRect().height, holds: scrolls.scrollHeight, line };
  }, inner);
}

test("ux_06 in a low window the open issue and Customize beside the list stay boxes as high as the list", async ({
  page,
}) => {
  await page.goto(`${projectIssuesAddress("WEB")}?open=WEB-1`);
  const panel = page.locator("[data-part~='side-panel']");
  // The sections of the issue come with their reads, held until all are there: the issue is whole with its History.
  await expect(panel.locator("[data-part='history.entry']").first()).toBeVisible();
  const open = await besideList(panel, "side-panel.body");
  expect(open.high).toBeLessThanOrEqual(open.line);
  expect(open.holds).toBeGreaterThan(open.high);
  await page.goto(projectIssuesAddress("WEB"));
  await page.getByRole("button", { name: t("issues.customize"), exact: true }).click();
  const columns = await besideList(page.locator("[data-part~='list-panel']"), "list-panel");
  expect(columns.high).toBeLessThanOrEqual(columns.line);
  expect(columns.holds).toBeGreaterThan(columns.high);
});

test.describe("a long list in a low window", () => {
  /** A long list: far more rows than any window shows. */
  const MANY = 2000;

  test.use({ preparation: { run: (server) => prepareManyWeb(server, MANY) } });

  test("ux_06 in a low window a long list still draws only the rows in view", async ({ page }) => {
    test.setTimeout(600_000);
    await page.goto(projectIssuesAddress("WEB"));
    const rows = page.locator("[data-part='list-scroll'] [data-part='list-row']");
    await expect(rows.first()).toBeVisible();
    expect(await rows.count(), "the rows drawn of the long list").toBeLessThan(MANY / 20);
    // The box of the rows stays a box: no higher than the window, and its rows scroll inside it.
    const box = await page.locator("[data-part='list-scroll']").evaluate((node) => ({
      high: node.clientHeight,
      holds: node.scrollHeight,
      window: innerHeight,
    }));
    expect(box.high).toBeLessThanOrEqual(box.window);
    expect(box.holds).toBeGreaterThan(box.high);
  });
});
