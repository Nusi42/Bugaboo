import { type Page } from "@playwright/test";

import { t } from "@bugaboo/messages";

import { ada } from "./support/accounts.ts";
import { signedInAdmin } from "./support/access.ts";
import { expect, refusal, test } from "./support/fixtures.ts";
import { openIssueWithDetails, signIn } from "./support/flows.ts";
import { issueOf, prepareWeb, conflictNote, versionOf } from "./support/issues.ts";
import { partIn } from "./support/parts.ts";

// A title changed in place that the server refuses (ISS-03, UX §6.4): the refused text goes, the
// title it had shows again, its button is named by what it shows, and the page says why.

test.use({ preparation: { run: prepareWeb } });
test.describe.configure({ mode: "serial" });

/** Types a new title and presses Enter; the title the heading had before. */
async function typeTitle(page: Page, title: string): Promise<string> {
  const heading = partIn(page, "issue-title");
  const before = (await heading.textContent()) ?? "";
  await heading.getByRole("button").click();
  await heading.getByRole("textbox").fill(title);
  await heading.getByRole("textbox").press("Enter");
  return before;
}

/** The title shows the old text again, and its button is named by what it shows. */
async function expectTitleBack(page: Page, before: string, refused: string): Promise<void> {
  const heading = partIn(page, "issue-title");
  await expect(heading).toHaveText(before);
  await expect(heading.getByRole("button")).toHaveAccessibleName(
    t("issues.change-title-of", { title: before }),
  );
  await expect(page.getByText(refused, { exact: true })).toHaveCount(0);
}

test("iss_03 a title the server refuses for want of rights goes, the old one shows and the page says why", async ({
  page,
}) => {
  await signIn(page, ada);
  await openIssueWithDetails(page, "WEB-1");
  await page.waitForLoadState("networkidle");
  await page.route(
    "**/api/v1/issues/*",
    async (route) => {
      await route.fulfill(refusal(403, "access.forbidden"));
    },
    { times: 1 },
  );
  const refused = "A title nobody may give";
  const before = await typeTitle(page, refused);
  await expect(
    page.getByRole("alert").filter({ hasText: t("error.access.forbidden") }),
  ).toBeVisible();
  await expectTitleBack(page, before, refused);
});

test("iss_03 a title on a version somebody changed first goes, the old one shows and the page offers the ways out", async ({
  page,
  server,
}) => {
  const admin = await signedInAdmin(server);
  const issue = await issueOf(admin, "WEB-1");
  await signIn(page, ada);
  await openIssueWithDetails(page, "WEB-1");
  await page.waitForLoadState("networkidle");
  // Somebody else changes the priority while the page is open.
  await admin.client.PATCH("/api/v1/issues/{issue}", {
    params: { path: { issue: issue.key } },
    body: { version: await versionOf(admin, issue), priority: "critical" },
  });
  const refused = "A title on an old version";
  const before = await typeTitle(page, refused);
  await expect(conflictNote(page)).toBeVisible();
  await expectTitleBack(page, before, refused);
});
