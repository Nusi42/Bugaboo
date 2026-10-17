import { ada } from "./support/accounts.ts";
import { test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { screenshotFolder } from "./support/screenshots.ts";
import { prepareWorkflows } from "./support/workflows.ts";
import { shootEditor } from "./support/workflows-shots.ts";
import { shootManyIssues } from "./support/workflows-shots-many.ts";
import { shootStatusChanges } from "./support/workflows-shots-status.ts";

// The screens of M4 WP-07 for the maintainer's review, each in the four themes, on a computer and on a
// phone (the Playwright project says which). They are only taken when `BB_SCREENSHOT_DIR` names
// the folder they go to:
//   BB_SCREENSHOT_DIR=... npx playwright test screenshots-workflows.spec --project=desktop --project=phone
test.skip(screenshotFolder === undefined, "Set BB_SCREENSHOT_DIR to take the screenshots.");
test.use({ preparation: { run: prepareWorkflows } });
test.describe.configure({ mode: "serial" });

/** Four themes of a screen or more take longer than an ordinary test. */
const TAKING_MS = 300_000;

test("the editor, its rules, the drawing and Try it", async ({ page }) => {
  test.setTimeout(TAKING_MS);
  await signIn(page, ada);
  await shootEditor(page);
});

test("the status button, Ask for and the question of Publish", async ({ page }) => {
  test.setTimeout(TAKING_MS);
  await signIn(page, ada);
  await shootStatusChanges(page);
});

test("changing many issues follows the workflow", async ({ page, server }) => {
  test.setTimeout(TAKING_MS);
  await signIn(page, ada);
  await shootManyIssues(page, server);
});
