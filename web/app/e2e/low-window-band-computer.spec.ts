import type { Locator, Page } from "@playwright/test";

import { t } from "@bugaboo/messages";

import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";
import { expectStill, movementWhile, partLandmark } from "./support/movement.ts";
import { readEverythingAgain } from "./support/read-again.ts";
import { scrollingOf } from "./support/scrolling.ts";
import { issueAddress, paths, projectIssuesAddress } from "../src/shared/paths.ts";

// A window in the band of a low one (UX §6.5): a laptop of 1366 by 650 is low by `size-low-window`, yet the head of a
// pane and the least of its boxes fit it. The list still fills the rest of the window, no box scrolls inside another,
// the page does not scroll, and a read again moves nothing; the panes beside the list and the editors of a list keep
// their room.

test.use({ preparation: { run: prepareWeb } });

/** A laptop with the browser's bars: lower than `size-low-window`, higher than any head with the least of its box. */
const LAPTOP = { width: 1366, height: 650 } as const;

test.beforeEach(async ({ page }) => {
  await page.setViewportSize(LAPTOP);
  await signIn(page, ada);
});

/** How far the bottom of a box stands from the bottom of the window. */
function roomUnder(box: Locator): Promise<number> {
  return box.evaluate((node) => innerHeight - node.getBoundingClientRect().bottom);
}

/** How far apart the bottom of a pane beside the list and the bottom of the box of the rows are. */
async function bottomsApart(side: Locator, page: Page): Promise<number> {
  const [aside, rows] = await Promise.all([
    side.evaluate((node) => node.getBoundingClientRect().bottom),
    page
      .locator("[data-part~='list-scroll']")
      .evaluate((node) => node.getBoundingClientRect().bottom),
  ]);
  return Math.abs(aside - rows);
}

/** One box per pane, the page still, and nothing moves while every read of the screen is made again. */
async function calmWith(page: Page, part: string): Promise<void> {
  // The first read of the screen is drawn whole before it is read again.
  await page.waitForLoadState("networkidle");
  expect(await scrollingOf(page)).toEqual({ page: 0, nested: [] });
  const read = await movementWhile(page, [partLandmark(part, part)], () =>
    readEverythingAgain(page),
  );
  expectStill(read);
}

test("ux_06 in the band of a low window the list fills the rest of it and stays still", async ({
  page,
}) => {
  await page.goto(projectIssuesAddress("WEB"));
  const rows = page.locator("[data-part~='list-scroll']");
  await expect(rows).toBeVisible();
  // The box of the rows reaches down to the foot of the card, not three rows and empty room.
  expect(await roomUnder(rows)).toBeLessThan(64);
  await calmWith(page, "list-scroll");
});

test("ux_06 in the band of a low window an issue and the timeline keep one box each and stay still", async ({
  page,
}) => {
  await page.goto(issueAddress("WEB-1"));
  await expect(page.locator("[data-part~='issue-main']")).toBeVisible();
  await calmWith(page, "issue-main");
  // The timeline of the project, as the list opens it.
  await page.goto(`${projectIssuesAddress("WEB")}?show=timeline`);
  await expect(page.locator("[data-part~='timeline-scroll']")).toBeVisible();
  await calmWith(page, "timeline-scroll");
});

test("ux_06 in the band of a low window the open issue beside the list ends with the rows and scrolls inside", async ({
  page,
}) => {
  await page.goto(`${projectIssuesAddress("WEB")}?open=WEB-1`);
  const panel = page.locator("[data-part~='side-panel']");
  const body = panel.locator("[data-part~='side-panel.body']");
  await expect(body).toBeVisible();
  await expect.poll(() => bottomsApart(panel, page)).toBeLessThanOrEqual(2);
  expect(await body.evaluate((node) => node.scrollHeight > node.clientHeight)).toBe(true);
});

test("ux_06 in the band of a low window Customize beside the list ends with the rows", async ({
  page,
}) => {
  await page.goto(projectIssuesAddress("WEB"));
  await page.getByRole("button", { name: t("issues.customize"), exact: true }).click();
  const panel = page.getByRole("complementary", {
    name: t("issues.customize"),
  });
  await expect(panel).toBeVisible();
  // Once the panel stands in its place beside the list.
  await expect.poll(() => bottomsApart(panel, page)).toBeLessThanOrEqual(2);
});

/** The height of a box, as it stands now. */
function heightOf(box: Locator): Promise<number> {
  return box.evaluate((node) => node.getBoundingClientRect().height);
}

/**
 * Opens Customize on the address and gives the panel, once the head of the list is read whole: the time of the project
 * is its last line to come, and the panel keeps the room of the whole head above it.
 */
async function openCustomize(page: Page, address: string): Promise<Locator> {
  await page.goto(address);
  await page.getByRole("button", { name: t("issues.customize"), exact: true }).click();
  const panel = page.getByRole("complementary", {
    name: t("issues.customize"),
  });
  await expect(panel).toBeVisible();
  await expect(page.getByText(t("issues.project-time"), { exact: false })).toBeVisible();
  return panel;
}

/** A search that finds nothing. */
const NOTHING_FOUND = "?s=title+contains+zzzzqq";

/** Customize is more than its padding and border: its head and several of its columns show. */
const FOLDED = 60;

/** Holds back the list of the issues until the function it returns is called. */
async function holdBackRows(page: Page): Promise<() => void> {
  const { promise: held, resolve: release } = Promise.withResolvers<void>();
  await page.route("**/api/v1/search/issues?*", async (route) => {
    await held;
    await route.continue();
  });
  return release;
}

/** Beside the list Customize also ends with the rows; under it, it is as high as its columns. */
const ENDS_WITH_ROWS = {
  beside: async (panel: Locator, page: Page) => {
    await expect.poll(() => bottomsApart(panel, page)).toBeLessThanOrEqual(2);
  },
  under: () => Promise.resolve(),
};

const WIDTHS = [
  ["beside", { width: 1366, height: 650 }],
  ["under", { width: 900, height: 600 }],
] as const;

test("ux_06 in a low window Customize under the list keeps room for its columns", async ({
  page,
}) => {
  await page.setViewportSize({ width: 900, height: 600 });
  const panel = await openCustomize(page, projectIssuesAddress("WEB"));
  const head = panel.locator("[data-part~='list-panel.head']");
  // Not folded to its padding: its head and several of its columns show, at once.
  const [panelHigh, headHigh] = await Promise.all([heightOf(panel), heightOf(head)]);
  expect(panelHigh / headHigh).toBeGreaterThan(3);
});

for (const [place, size] of WIDTHS) {
  test(`ux_06 in a low window Customize ${place} a list with nothing in it is as high as beside rows`, async ({
    page,
  }) => {
    await page.setViewportSize(size);
    const withRows = await heightOf(await openCustomize(page, projectIssuesAddress("WEB")));
    const panel = await openCustomize(page, `${projectIssuesAddress("WEB")}${NOTHING_FOUND}`);
    await expect(page.getByText(t("search.found-none"), { exact: true })).toBeVisible();
    expect(await heightOf(panel)).toBeGreaterThan(FOLDED);
    expect(await heightOf(panel)).toBe(withRows);
  });

  test(`ux_06 in a low window Customize ${place} a list still loading keeps one height when the rows come`, async ({
    page,
  }) => {
    await page.setViewportSize(size);
    const release = await holdBackRows(page);
    const panel = await openCustomize(page, projectIssuesAddress("WEB"));
    await expect(page.locator("[data-part~='list-scroll']")).toHaveCount(0);
    const before = await heightOf(panel);
    expect(before).toBeGreaterThan(FOLDED);
    release();
    await expect(page.locator("[data-part~='list-scroll']")).toBeVisible();
    expect(await heightOf(panel)).toBe(before);
    await ENDS_WITH_ROWS[place](panel, page);
  });
}

test("ux_06 in a low window an editor of a list keeps the room of the item it opens", async ({
  page,
}) => {
  await page.setViewportSize({ width: 1280, height: 650 });
  await page.goto(paths.projects);
  await page.locator("[data-part~='list-editor.open']").first().click();
  const panel = page.locator("[data-part~='side-panel']");
  const [head, body] = [
    panel.locator("[data-part~='side-panel.head']"),
    panel.locator("[data-part~='side-panel.body']"),
  ];
  await expect(body).toBeVisible();
  const [headHigh, bodyHigh] = await Promise.all([
    head.evaluate((node) => node.getBoundingClientRect().height),
    body.evaluate((node) => node.getBoundingClientRect().height),
  ]);
  expect(bodyHigh).toBeGreaterThan(headHigh);
});
