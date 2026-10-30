import { t } from "@bugaboo/messages";
import { type Locator } from "@playwright/test";

import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareWeb, webIssues } from "./support/issues.ts";
import { issueAddress, paths } from "../src/shared/paths.ts";

// No word alone on the last line (M5 exit): the title of an issue, the intro of a page and every other text wrap so
// that the last line keeps more than one word; the page sets it once and every text inherits it.

test.use({ preparation: { run: prepareWeb } });

/** How the browser wraps the text of an element. */
function wrapOf(element: Locator): Promise<string> {
  return element.evaluate((node) => getComputedStyle(node).textWrapStyle);
}

test("ux_7_3 the title of an issue and the intro of a page leave no word alone on their last line", async ({
  page,
}) => {
  await signIn(page, ada);
  await page.goto(issueAddress("WEB-1"));
  const title = page.getByRole("heading", { level: 1, name: webIssues.login });
  await expect(title).toBeVisible();
  expect(await wrapOf(title)).toBe("pretty");
  await page.goto(paths.region);
  const intro = page.getByText(t("setting.page.region-intro"), { exact: true });
  await expect(intro).toBeVisible();
  expect(await wrapOf(intro)).toBe("pretty");
});
