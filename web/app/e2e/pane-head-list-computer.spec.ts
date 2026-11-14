import type { Page } from "@playwright/test";

import { t } from "@bugaboo/messages";

import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareManyWeb } from "./support/issues.ts";
import { inOrder } from "./support/sequence.ts";
import { expectStill, movementWhile, partLandmark } from "./support/movement.ts";
import {
  coverColors,
  coveredSteps,
  firstRowGap,
  focusInHead,
  foldsWith,
  keyThrough,
  LOW_WINDOW,
  wheelOver,
} from "./support/pane-head.ts";
import { projectIssuesAddress } from "../src/shared/paths.ts";

// The head of a list (the toolbar, the search bar, the options and the counts) folds to its toolbar as the rows scroll
// and opens again at their top (UX §6.5); the head of the columns goes up with it, and nothing else moves.

/** More rows than the low window shows. */
const MANY = 40;

test.use({ preparation: { run: (server) => prepareManyWeb(server, MANY) } });

test("ux_01 the head of a list folds as its rows scroll and nothing else moves in any frame", async ({
  page,
}) => {
  await page.setViewportSize(LOW_WINDOW);
  await signIn(page, ada);
  await page.goto(projectIssuesAddress("WEB"));
  const rows = page.locator("[data-part='list-scroll']");
  await expect(rows).toBeVisible();
  await page.waitForLoadState("networkidle");
  const movement = await movementWhile(
    page,
    [
      partLandmark("the heading of the card", "card.heading"),
      partLandmark("the toolbar", "list-toolbar"),
      partLandmark("the rows", "list-scroll"),
    ],
    () => wheelOver(page, rows),
  );
  // The head folds and opens again with the scroll; the heading and the toolbar, its first line, stay where they are.
  expectStill(movement, { scrolls: ["list-scroll"] });
  // The whole head (the search bar and the options of the list are in it once the list is read) folds and opens.
  await foldsWith(page, rows);
});

/** The box of the rows. */
const ROWS = "[data-part='list-scroll']";

/** A window as high as a laptop's, for the keyboard and for paper. */
const LAPTOP = { width: 1280, height: 620 } as const;

/** Opens the list of the project in the window and waits until it is read whole. */
async function openList(page: Page, size: typeof LAPTOP): Promise<void> {
  await page.setViewportSize(size);
  await signIn(page, ada);
  await page.goto(projectIssuesAddress("WEB"));
  await expect(page.locator(ROWS)).toBeVisible();
  await page.waitForLoadState("networkidle");
}

test("ux_06 the arrow keys bring a row out from under the folded head and the head of the columns", async ({
  page,
}) => {
  await openList(page, LAPTOP);
  await page.locator(`${ROWS} [role='row']`).first().focus();
  const down = await keyThrough(page, "ArrowDown", 20, ROWS, "[data-part='list-head']");
  const up = await keyThrough(page, "ArrowUp", 14, ROWS, "[data-part='list-head']");
  expect(coveredSteps([...down, ...up]), "the rows the heads covered").toEqual([]);
  // The first row brings the box back to its start, with the head whole.
  await page.keyboard.press("Home");
  await expect.poll(() => page.locator(ROWS).evaluate((node) => node.scrollTop)).toBe(0);
});

test("ux_06 on paper the first row stands under the head of the columns as on the screen", async ({
  page,
}) => {
  await openList(page, LAPTOP);
  const gap = () => firstRowGap(page);
  // On the screen the rows keep the room of the whole head above them, under the head laid over them; on paper the
  // head stands in the page and keeps no room.
  const onScreen = await gap();
  await page.emulateMedia({ media: "print" });
  await expect.poll(gap).toBeCloseTo(onScreen, 0);
});

for (const scheme of ["light", "dark"] as const) {
  test(`ux_06 the head of the list covers the rows with the color of its card (${scheme})`, async ({
    page,
  }) => {
    await page.emulateMedia({ colorScheme: scheme });
    await openList(page, LAPTOP);
    const colors = await coverColors(page);
    expect(colors.head).toBe(colors.card);
  });
}

test("ux_06 a control of the folded head of a list reached with the keyboard shows whole, over the head of the columns", async ({
  page,
}) => {
  await openList(page, LAPTOP);
  await page.locator(ROWS).evaluate((node) => {
    node.scrollTop = node.scrollHeight;
  });
  // From the first line of the folded head (the toolbar) down through the search bar and the options.
  await page
    .getByRole("button", { name: t("issues.new"), exact: true })
    .first()
    .focus();
  const seen: Awaited<ReturnType<typeof focusInHead>>[] = [];
  await inOrder(
    Array.from({ length: 6 }, () => async () => {
      await page.keyboard.press("Tab");
      seen.push(await focusInHead(page));
    }),
  );
  expect(seen).toEqual(seen.map(() => ({ inside: true, onTop: true, scrolled: 0 })));
});
