import { t } from "@bugaboo/messages";

import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { sampleAdministrator } from "./support/sample.ts";
import { inOrder } from "./support/sequence.ts";
import {
  actionsOfMore,
  actionsOfSwipe,
  swipe,
  swipeClosed,
  swipedActions,
  swipeStrip,
} from "./support/swipe.ts";
import { firstRow, prepareAgent, swipedLists } from "./support/swipe-lists.ts";
import { boxOf } from "./support/windows.ts";
import { projectIssuesAddress } from "../src/shared/paths.ts";
import { viewports } from "../../tooling/viewports.ts";

// Swipe actions on the rows of the lists of a phone (UX-02, UX §3.3, principle 9): a swipe brings in at most two
// actions at a side, and the row's *More* holds the same ones, so nothing depends on a swipe. With *Less motion* the
// actions are there at once, without sliding in.

test.use({ sample: true });

test("ux_02_a_swipe_on_a_list_row_offers_the_same_actions_as_more", async ({ page, server }) => {
  await prepareAgent(server);
  await signIn(page, sampleAdministrator());
  await inOrder(
    swipedLists.map((list) => async () => {
      const row = await list.open(page);
      const swiped: string[] = [];
      await inOrder(
        list.swipes.map((towards) => async () => {
          swiped.push(...(await actionsOfSwipe(page, row, towards)));
        }),
      );
      expect(swiped.length, list.name).toBeGreaterThan(0);
      expect(await actionsOfMore(page, row), list.name).toEqual(swiped);
    }),
  );
});

test("ux_02_less_motion_shows_swipe_actions_without_sliding", async ({ browser, server }) => {
  const context = await browser.newContext({
    baseURL: server.url,
    ignoreHTTPSErrors: true,
    viewport: viewports.phone,
    hasTouch: true,
    reducedMotion: "reduce",
  });
  const page = await context.newPage();
  await signIn(page, sampleAdministrator());
  const row = await firstRow(page, projectIssuesAddress("WEB"), "list-row");
  await swipe(page, row, "start");
  const strip = swipeStrip(row);
  await expect(swipedActions(row).first()).toBeVisible();
  // No slide: the actions stand at the end edge of the row from the first picture on.
  expect(await strip.evaluate((node) => getComputedStyle(node).animationDuration)).toBe("0s");
  const [rowBox, stripBox] = [await boxOf(row), await boxOf(strip)];
  expect(Math.abs(rowBox.x + rowBox.width - (stripBox.x + stripBox.width))).toBeLessThanOrEqual(1);
  await context.close();
});

test("ux_02 the swipe actions of an issue do what they say: Follow, and the change of status of that one issue", async ({
  page,
}) => {
  await signIn(page, sampleAdministrator());
  const row = await firstRow(page, projectIssuesAddress("WEB"), "list-row");
  const words = [t("followers.follow"), t("followers.stop")];
  // A finger taps, as on a phone: the side closes and the person follows the issue, or stops.
  await swipe(page, row, "end");
  const before = (await swipedActions(row).first().innerText()).trim();
  await swipedActions(row).first().tap();
  await expect(swipeStrip(row)).toHaveCount(0);
  await swipe(page, row, "end");
  expect(words).toContain(before);
  await expect(swipedActions(row).first()).toHaveText(words.filter((word) => word !== before));
  await swipeClosed(page, row, "end");
  await swipe(page, row, "start");
  await swipedActions(row).getByText(t("issues.row-change-status"), { exact: true }).tap();
  const dialog = page.getByRole("alertdialog", {
    name: t("many-changes.title", { action: t("many-changes.action-change"), count: 1 }),
  });
  await expect(dialog).toBeVisible();
  await dialog.getByRole("button", { name: t("common.cancel"), exact: true }).tap();
  await expect(dialog).toBeHidden();
});
