import { type Locator, type Page } from "@playwright/test";

import { issueAddress, paths, projectIssuesAddress } from "../src/shared/paths.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { partIn } from "./support/parts.ts";
import { ask, list } from "./support/quick-search.ts";
import { sampleAdministrator } from "./support/sample.ts";
import { ISSUE, ISSUE_PROJECT } from "./support/showcase-sample.ts";
import { tokenValue } from "./support/tokens.ts";

// M5-F137: an issue ID is in the code font everywhere it is shown, drawn by the one part `issue-id`: in the list, on
// the issue page, in a notification and in Quick search.

test.use({ sample: true });

/** A font stack with its names as words, whether the browser quotes a name or not. */
function namesOf(stack: string): readonly string[] {
  return stack.split(",").map((name) => name.trim().replaceAll(/["']/gu, ""));
}

/** The font stack of code and IDs, as the theme gives it to the page. */
async function codeFont(page: Page): Promise<readonly string[]> {
  return namesOf(await tokenValue(page, "--font-mono"));
}

/** The font a drawn ID is in. */
async function fontOf(id: Locator): Promise<readonly string[]> {
  return namesOf(await id.evaluate((node) => getComputedStyle(node).fontFamily));
}

/** The first ID drawn in a place is in the code font. */
async function expectCodeFont(page: Page, place: Locator, where: string): Promise<void> {
  const id = partIn(place, "issue-id").first();
  await expect(id, where).toBeVisible();
  expect(await fontOf(id), where).toEqual(await codeFont(page));
}

test("iss_01 an issue id is in the code font everywhere", async ({ page }) => {
  await signIn(page, sampleAdministrator());

  await page.goto(projectIssuesAddress(ISSUE_PROJECT));
  await expectCodeFont(page, page.getByRole("main"), "the list");

  await page.goto(issueAddress(ISSUE));
  await expectCodeFont(page, page.getByRole("main"), "the issue page");

  await page.goto(paths.notifications);
  await expectCodeFont(page, page.getByRole("main"), "a notification");

  await ask(page, ISSUE);
  await expectCodeFont(page, list(page), "Quick search");
});
