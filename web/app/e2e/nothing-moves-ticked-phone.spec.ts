import { t } from "@bugaboo/messages";

import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareManyWeb } from "./support/issues.ts";
import { listLandmarks, ROWS_BOX } from "./support/landmarks.ts";
import { expectNotSmaller, expectStill, movementWhile } from "./support/movement.ts";
import { lastRowEndAndBarTop, scrollRowsToTheEnd } from "./support/rows-end.ts";
import { projectIssuesAddress } from "../src/shared/paths.ts";

// The bar of the ticked issues on a phone (UX §4.2, §6.4, M8 WP-13): it slides up over the foot of
// the list, nothing in the list moves, and the list gains room at its end, so its last row can be
// scrolled above the bar.

test.use({ preparation: { run: (server) => prepareManyWeb(server, 20) } });

test("ux_01 ticking an issue on a phone moves no row, and the last row can be scrolled above the bar", async ({
  page,
}) => {
  await signIn(page, ada);
  await page.goto(projectIssuesAddress("WEB"));
  const rows = page.locator("[data-part='list-scroll']");
  await expect(rows.getByRole("row").first()).toBeVisible();
  await page.waitForLoadState("networkidle");
  // The box of the first issue in view: the person ticks what they see.
  const box = rows
    .locator("label")
    .filter({ has: page.getByRole("checkbox") })
    .first();
  const list = await listLandmarks(page);
  const ticking = await movementWhile(page, list.all, async () => {
    await box.click();
    await expect(
      page.getByRole("status").filter({ hasText: t("issues.selected", { count: 1 }) }),
    ).toBeVisible();
  });
  expectStill(ticking, list);
  // The bar takes no room from the rows.
  expectNotSmaller(ticking, ROWS_BOX.name);
  // At the end of the list the last row stands whole above the bar.
  const bar = page.getByRole("region", { name: t("issues.selected-bar"), exact: true });
  await scrollRowsToTheEnd(rows);
  const { rowEnd, barTop } = await lastRowEndAndBarTop(rows, bar);
  expect(rowEnd).toBeLessThanOrEqual(barTop + 0.5);
});
