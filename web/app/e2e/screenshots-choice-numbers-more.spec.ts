import { paths } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { prepareBudgets } from "./support/budget.ts";
import { inFourThemesOf, openTheFilter } from "./support/choice-pictures.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { screenshotFolder } from "./support/screenshots.ts";

// The search by the number of a choice in the four themes, for the review of M6. Only taken when
// `BB_SCREENSHOT_DIR` names the folder they go to.
test.skip(screenshotFolder === undefined, "Set BB_SCREENSHOT_DIR to take the screenshots.");
test.use({ preparation: { run: prepareBudgets } });

/** Where the pictures go, below the folder the run was given. */
const MILESTONE = "M6/choice-numbers";

test("a search by the number of a choice, and by the order of the numbers", async ({ page }) => {
  await signIn(page, ada);
  const search = (text: string, found: string) => async () => {
    await page.goto(`${paths.search}?s=${encodeURIComponent(text)}`);
    await expect(page.getByText(found, { exact: false }).first()).toBeVisible();
  };
  const main = () => page.getByRole("main");
  await inFourThemesOf(
    page,
    main,
    { name: "search-number", milestone: MILESTONE, toContent: true },
    search("budget is 4100", "Spent on Product development"),
  );
  await inFourThemesOf(
    page,
    main,
    { name: "search-after-number", milestone: MILESTONE, toContent: true },
    search("budget > 4100", "Spent on Sales"),
  );
  // The window that changes the filter, opened from its button.
  await inFourThemesOf(
    page,
    () => page.locator("[data-part='overlay.modal']").last(),
    { name: "search-filter-open", milestone: MILESTONE, asIs: true },
    async () => {
      await search("budget is 4100", "Spent on Product development")();
      await openTheFilter(page);
    },
  );
});
