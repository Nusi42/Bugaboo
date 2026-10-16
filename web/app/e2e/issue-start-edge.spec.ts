import { t } from "@bugaboo/messages";

import { issueAddress } from "../src/shared/paths.ts";
import { expect, test } from "./support/fixtures.ts";
import { detailRow } from "./support/issues.ts";
import { layoutOf } from "./support/layout.ts";
import { openList, prepareLook } from "./support/list-look.ts";
import { partIn } from "./support/parts.ts";
import { pixelsOf } from "./support/tokens.ts";
import { boxOf } from "./support/windows.ts";

// The text column is at most as wide as the theme file says (UX §4.4). One start edge (R1, M3 WP-17 review): a button that holds text pulls itself back by its own
// padding, so the text of the title and of the quiet buttons stands where the other text starts.

test.use({ preparation: { run: prepareLook } });

test("ux_4_4 R1 the title and the quiet buttons of the Details start where the text starts", async ({
  page,
}) => {
  test.skip(layoutOf(page) !== "computer", "the Details are a column on a computer");
  await openList(page);
  await page.goto(issueAddress("WEB-1"));
  // A button that holds text pulls itself back by its own padding, so its text stands on the edge.
  const title = partIn(page, "in-place.button").first();
  await expect(title).toHaveCSS("margin-inline-start", "-8px");
  const change = detailRow(page, t("issue-field.assigned-to")).getByRole("button");
  await expect(change).toHaveCSS("margin-inline-start", "-12px");
});

test("ux_4_4 the text column is at most 780 wide", async ({ page }) => {
  test.skip(layoutOf(page) !== "computer", "the columns are for a computer");
  await openList(page);
  await page.goto(issueAddress("WEB-1"));
  const main = await boxOf(partIn(page, "issue-main"));
  // The widest the text column of the issue page may be (UX §4.4) is a token of the theme file.
  const widest = await pixelsOf(page, "--size-main");
  expect(main.width).toBeLessThanOrEqual(widest);
  expect(main.width).toBeGreaterThan(widest / 2);
});
