import { t } from "@bugaboo/messages";

import { paths, runAddress } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { signedInAdmin } from "./support/access.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { inFourChosenThemes, inFourThemes } from "./support/four-themes.ts";
import { prepareWeb } from "./support/issues.ts";
import { commentOn, createAppSession, issueIdOf, markText } from "./support/runs.ts";
import { screenshotFolder } from "./support/screenshots.ts";

// The run page and the mark of AI-written text in the four themes, for the maintainer's review of M7.
// They are only taken when `BB_SCREENSHOT_DIR` names the folder they go to.
test.skip(screenshotFolder === undefined, "Set BB_SCREENSHOT_DIR to take the screenshots.");
test.use({ preparation: { run: prepareWeb } });
test.describe.configure({ mode: "serial" });

/** Where the pictures go, below the folder the run was given. */
const MILESTONE = "M7/runs";

/** Four themes of a screen take longer than an ordinary test. */
const TAKING_MS = 300_000;

test("the page of a run before and after the look and the undo", async ({ page, server }) => {
  test.setTimeout(TAKING_MS);
  const admin = await signedInAdmin(server);
  const run = await createAppSession(server, admin, [
    { issue: { id: await issueIdOf(admin, "WEB-1"), key: "WEB-1" }, version: 1 },
    { issue: { id: await issueIdOf(admin, "WEB-2"), key: "WEB-2" }, version: 1 },
  ]);
  await admin.dispose();
  await signIn(page, ada);
  await page.goto(runAddress(run));
  await expect(page.getByText("WEB-1", { exact: true })).toBeVisible();
  await inFourThemes(page, "run-before", MILESTONE);
  await page.getByRole("button", { name: t("runs.look-first"), exact: true }).click();
  await expect(page.getByText(t("runs.preview", { back: 2, skipped: 0 }))).toBeVisible();
  await inFourThemes(page, "run-preview", MILESTONE);
  await page.getByRole("button", { name: t("runs.undo"), exact: true }).click();
  await expect(page.getByText(t("runs.undone", { back: 2, skipped: 0 }))).toBeVisible();
  await inFourThemes(page, "run-undone", MILESTONE);
});

test("the mark beside a comment and the description", async ({ page, server }) => {
  test.setTimeout(TAKING_MS);
  const admin = await signedInAdmin(server);
  const issue = await issueIdOf(admin, "WEB-1");
  const comment = await commentOn(admin, "WEB-1", "Looked at it and found the cause.");
  await admin.dispose();
  markText(server, {
    kind: "issue",
    id: issue,
    part: "description",
    writtenBy: "assistant",
  });
  markText(server, {
    kind: "comment",
    id: comment,
    part: "text",
    writtenBy: "app",
    app: "Claude Code",
  });
  await signIn(page, ada);
  await inFourChosenThemes(page, "issue-mark", MILESTONE, paths.appearance, async () => {
    await page.goto("/issues/WEB-1");
    await expect(page.getByText(t("written-by.assistant"), { exact: true })).toBeVisible();
    await expect(
      page.getByText(t("written-by.app", { app: "Claude Code" }), { exact: true }),
    ).toBeVisible();
  });
});
