import { t } from "@bugaboo/messages";

import { paths } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { prepareBudgets } from "./support/budget.ts";
import { openTheFilter } from "./support/choice-pictures.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";

// The window that changes a filter by a numbered choice writes the choice as every screen does,
// "4100 Product development", and offers each choice once (TIME-13, UX §7.3).

test.use({ preparation: { run: prepareBudgets } });

test("time_13 the value box of a filter by a numbered choice shows the number with the name", async ({
  page,
}) => {
  await signIn(page, ada);
  await page.goto(`${paths.search}?s=${encodeURIComponent("budget is 4100")}`);
  await openTheFilter(page);
  const box = page
    .locator("[data-part='overlay.modal']")
    .last()
    .getByLabel(t("search.bar.value"), { exact: true });
  // The number alone is how the filter was written, the box still shows the choice whole.
  await expect(box.locator("option:checked")).toHaveText("4100 Product development");
  // Each choice is offered once, the number and the name together.
  const offered = await box.getByRole("option").allTextContents();
  expect(offered).toContain("4100 Product development");
  expect(offered).not.toContain("4100");
  expect(offered).not.toContain("Product development");
});
