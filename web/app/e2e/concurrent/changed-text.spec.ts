import { t } from "@bugaboo/messages";

import { signedInAdmin } from "../support/access.ts";
import { ada } from "../support/accounts.ts";
import { expectGoodInEveryLook } from "../support/checks.ts";
import { signedInPage } from "../support/crowd-pages.ts";
import { ben, crowdIssues, prepareCrowd } from "../support/crowd.ts";
import { startEditing } from "../support/description.ts";
import { expect, test } from "../support/fixtures.ts";
import { issueOf, saveDescription, versionOf } from "../support/issues.ts";

// Somebody saves the description while a person has the box open (ISS-17, M3-F54): the page says
// who changed it, and *Show changes* marks the words that differ.

test.use({ preparation: { run: prepareCrowd } });

test("iss_17_show_changes_says_who_changed_the_text_and_marks_the_words_that_differ", async ({
  browser,
  server,
  viewport,
  hasTouch,
}) => {
  const admin = await signedInAdmin(server);
  const issue = await issueOf(admin, crowdIssues.description);
  await saveDescription(admin, issue, "The login fails on Monday morning.", 1);
  const reader = await signedInPage(browser, server, ben, { viewport, hasTouch });
  await startEditing(reader, crowdIssues.description);
  // Somebody else saves a changed text, and the person comes back to the tab.
  await saveDescription(
    admin,
    issue,
    "The login fails on Tuesday morning.",
    await versionOf(admin, issue),
  );
  await reader.evaluate(() => window.dispatchEvent(new Event("focus")));
  await expect(
    reader.getByText(`${ada.name} changed the description after you started.`, { exact: true }),
  ).toBeVisible();
  await reader.getByRole("button", { name: t("issues.show-changes"), exact: true }).click();
  // Only the words that differ are marked, on both sides.
  await expect(reader.locator("mark", { hasText: "Tuesday" })).toBeVisible();
  await expect(reader.locator("mark", { hasText: "Monday" })).toBeVisible();
  await expect(reader.locator("mark", { hasText: "login" })).toHaveCount(0);
  await expectGoodInEveryLook(reader);
  await admin.dispose();
});
