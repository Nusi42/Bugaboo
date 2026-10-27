import { manyChangesIssuesMax } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";

import { projectIssuesAddress } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareManyWeb } from "./support/issues.ts";
import { layoutOf } from "./support/layout.ts";
import { rowOf } from "./support/list-look.ts";

// Long lists (M5 WP-08): "Tick all results" ticks the issues of a search that the page does not
// hold.

/** How many issues the project WEB has when the list is long: more than a page of the list. */
const MANY = 130;

test.describe("a list a few pages long", () => {
  test.use({ preparation: { run: (server) => prepareManyWeb(server, MANY) } });

  test("cfg_02 tick all results ticks up to five thousand", async ({ page }) => {
    test.setTimeout(120_000);
    test.skip(layoutOf(page) === "phone", "a phone ticks one card at a time");
    await signIn(page, ada);
    await page.goto(projectIssuesAddress("WEB"));
    await expect(rowOf(page, "Another issue 129")).toBeVisible();
    // The box in the head ticks the rows shown; the bar then offers every result.
    await page
      .getByRole("checkbox", { name: t("issues.select-shown"), exact: true })
      .check({ force: true });
    const total = MANY + 3;
    expect(total).toBeLessThanOrEqual(manyChangesIssuesMax);
    await page.getByRole("button", { name: t("issues.tick-all", { count: total }) }).click();
    await expect(
      page.getByRole("status").filter({ hasText: t("issues.selected", { count: total }) }),
    ).toHaveCount(1);
  });
});
