import { projectIssuesAddress } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { webIssues } from "./support/issues.ts";
import { layoutOf } from "./support/layout.ts";
import { rowOf } from "./support/list-look.ts";
import { listInFourThemes, screenshotFolder } from "./support/list-pictures.ts";
import { prepareFull } from "./support/list-pictures-data.ts";
import { sampleAdministrator } from "./support/sample.ts";

// More pictures of the issue list (M5 WP-08): the rows with people and labels, the offer to tick
// every result, and the sentence that asks to narrow the search. Taken only when
// `BB_SCREENSHOT_DIR` is set; the builder looks at each.

test.skip(screenshotFolder === undefined, "Set BB_SCREENSHOT_DIR to take the screenshots.");

test.describe("rows with people, labels and the type", () => {
  test.use({ preparation: { run: prepareFull } });

  test("iss_01 pictures: assignees, labels and the type in the rows", async ({ page }) => {
    await signIn(page, ada);
    await page.goto(
      `${projectIssuesAddress("WEB")}?cols=status,id,type,title,labels,priority,assignedTo,changed&group=-`,
    );
    await expect(rowOf(page, webIssues.login)).toBeVisible();
    await listInFourThemes(page, "people-labels");
  });
});

test.describe("rows of a phone with two people", () => {
  test.use({ sample: true });

  test("iss_01 pictures: a line of two people on a phone breaks only between the names", async ({
    page,
  }) => {
    test.skip(layoutOf(page) !== "phone", "the line of people wraps on a phone");
    await signIn(page, sampleAdministrator());
    await page.goto(
      `${projectIssuesAddress("SHOP")}?s=${encodeURIComponent('assigned to is "Niko Eriksen"')}`,
    );
    await expect(page.getByText(/\sand\s/u).first()).toBeVisible();
    await listInFourThemes(page, "people-two");
  });
});
