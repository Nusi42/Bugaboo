import { t } from "@bugaboo/messages";
import { type Page } from "@playwright/test";

import { ada } from "./support/accounts.ts";
import { signedInAdmin } from "./support/access.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareWeb, webIssues } from "./support/issues.ts";
import { issueAddress, projectIssuesAddress } from "../src/shared/paths.ts";

// Putting an issue into the trash from the issue page (ISS-01, M3-F151): the page goes back to the
// list and offers Undo; a delete that the server refuses stays on the page and says why.

test.use({ preparation: { run: prepareWeb } });
test.describe.configure({ mode: "serial" });

/** *More* of the issue (the header of the page), not the *More* of the main menu. */
async function openMore(page: Page): Promise<void> {
  await page
    .locator("article")
    .getByRole("button", { name: t("issues.more"), exact: true })
    .first()
    .click();
}

test("iss_01 deleting an issue goes back to the list and Undo brings it back", async ({ page }) => {
  await signIn(page, ada);
  await page.goto(issueAddress("WEB-2"));
  await openMore(page);
  await page.getByRole("button", { name: t("issues.delete"), exact: true }).click();
  await expect(page).toHaveURL(new RegExp(`${projectIssuesAddress("WEB")}$`, "u"));
  await expect(page.getByText(t("issues.deleted", { key: "WEB-2" }))).toBeVisible();
  await page.getByRole("button", { name: t("common.undo"), exact: true }).click();
  const list = page.getByRole("list", { name: t("issues.list"), exact: true });
  await expect(list.getByRole("link", { name: webIssues.passkey })).toBeVisible();
});

test("iss_01 a delete that the server refuses stays on the page and says why", async ({
  page,
  server,
}) => {
  await signIn(page, ada);
  await page.goto(issueAddress("WEB-1"));
  await expect(page.getByRole("heading", { name: new RegExp(webIssues.login, "u") })).toBeVisible();
  // Somebody else puts the issue into the trash first.
  const admin = await signedInAdmin(server);
  await admin.client.DELETE("/api/v1/issues/{issue}", { params: { path: { issue: "WEB-1" } } });
  await admin.dispose();
  await openMore(page);
  await page.getByRole("button", { name: t("issues.delete"), exact: true }).click();
  await expect(
    page.getByRole("alert").filter({ hasText: t("error.resource.not-found") }),
  ).toBeVisible();
  await expect(page).toHaveURL(new RegExp(`${issueAddress("WEB-1")}$`, "u"));
});
