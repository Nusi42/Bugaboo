import { t } from "@bugaboo/messages";
import { type Page } from "@playwright/test";

import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { openNewIssueDialog } from "./support/issues.ts";
import { layoutOf } from "./support/layout.ts";
import { sampleAdministrator } from "./support/sample.ts";
import {
  boxOf,
  inDock,
  minimizeADraft,
  pageColumn,
  titleBox,
  windowButton,
  windowDock,
} from "./support/windows.ts";
import { projectIssuesAddress } from "../src/shared/paths.ts";
import { viewports } from "../../tooling/viewports.ts";

// M5 exit, package O (O7): the bar of a minimized window stands at the bottom of the page, and the page has
// room for it at its end, so the last row of a list is not covered by it.

test.use({ sample: true });

/** Scrolls the page to its end, whichever of the page and its frame scrolls. */
async function scrollToEnd(page: Page): Promise<void> {
  await page.evaluate(() => {
    window.scrollTo(0, document.documentElement.scrollHeight);
  });
}

test("ux_8 the bar of a minimized window covers no row at the end of the list", async ({
  page,
}) => {
  test.skip(layoutOf(page) === "phone", "the bar of a phone stands above the bar of the app");
  await signIn(page, sampleAdministrator());
  await page.goto(projectIssuesAddress("WEB"));
  await expect(page.getByRole("grid").first()).toBeVisible();
  await openNewIssueDialog(page);
  await titleBox(page).fill("A draft");
  await windowButton(page, t("window.minimize")).click();
  await expect(windowDock(page)).toBeVisible();
  await scrollToEnd(page);
  const dock = await boxOf(windowDock(page));
  const content = await boxOf(pageColumn(page).column);
  const card = await boxOf(pageColumn(page).card);
  // At the end of the page the strip is under the content: no row and no frame of a card is covered.
  expect(content.y + content.height).toBeLessThanOrEqual(dock.y + 0.5);
  expect(card.y + card.height).toBeLessThanOrEqual(dock.y + 0.5);
  // Its bar starts where the page column starts and stays inside the column, so it sits under the cards, not across them.
  const title = await boxOf(inDock(page, "window-dock.title"));
  const note = await boxOf(inDock(page, "window-dock.note"));
  // (a few pixels of rounding of the centered column are allowed)
  expect(Math.abs(title.x - content.x)).toBeLessThanOrEqual(3);
  expect(note.x + note.width).toBeLessThanOrEqual(content.x + content.width + 0.5);
});

/** The size of a window of the browser. */
interface WindowSize {
  readonly width: number;
  readonly height: number;
}

/** A tall window of a computer, beside the standard one of 800 px. */
const tallWindow: WindowSize = { width: viewports.computer.width, height: 1100 };

/** The card of the list and its box of rows, at rest (the page not scrolled), in the window as it is now. */
async function listAtRest(page: Page) {
  const { card, rows } = pageColumn(page);
  await expect(rows).toBeVisible();
  return { card: await boxOf(card), rows: await boxOf(rows) };
}

/** Without the strip the card ends at most a gutter above the end of the window: the box of rows took the rest of it. */
async function expectListTakesTheRest(page: Page, size: WindowSize): Promise<void> {
  await page.setViewportSize(size);
  await page.goto(projectIssuesAddress("WEB"));
  const { card } = await listAtRest(page);
  const under = size.height - (card.y + card.height);
  expect(under, `${size.height}: room under the card`).toBeGreaterThanOrEqual(0);
  expect(under, `${size.height}: room under the card`).toBeLessThanOrEqual(24);
}

/** With the strip, its room is taken off once: the card and the rows end above it, both edges. The window is changed in
 * place, because a new page would forget the minimized window. */
async function expectListAboveStrip(page: Page, size: WindowSize): Promise<void> {
  await page.setViewportSize(size);
  const { card, rows } = await listAtRest(page);
  const dock = await boxOf(windowDock(page));
  expect(card.y + card.height, `${size.height}: the card`).toBeLessThanOrEqual(dock.y + 0.5);
  expect(rows.y + rows.height, `${size.height}: the rows`).toBeLessThanOrEqual(dock.y + 0.5);
}

test("ux_8 a list takes the rest of the window, and at rest its card and its rows end above the strip of a minimized window", async ({
  page,
}) => {
  test.skip(layoutOf(page) !== "computer", "the phone list has its own room above the bars");
  await signIn(page, sampleAdministrator());
  await expectListTakesTheRest(page, viewports.computer);
  await expectListTakesTheRest(page, tallWindow);
  await minimizeADraft(page);
  await expectListAboveStrip(page, viewports.computer);
  await expectListAboveStrip(page, tallWindow);
});
