import { t } from "@bugaboo/messages";
import type { Locator, Page } from "@playwright/test";

import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { anotherBrowser, signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";
import { issueAddress } from "../src/shared/paths.ts";

// Editing a comment under its lock (ISS-05, ISS-17, M3 WP-13): when the lock is somebody
// else's the person still sees the comment, is told why, and can close the note.

test.use({ preparation: { run: prepareWeb } });
test.describe.configure({ mode: "serial" });

/** Presses *Edit* of the comment until `result` shows: the list is read again after a change. */
async function pressEditUntil(page: Page, result: Locator): Promise<void> {
  await expect(async () => {
    await page.getByRole("button", { name: t("comments.edit"), exact: true }).click();
    await expect(result).toBeVisible({ timeout: 2000 });
  }).toPass();
}

test("iss_05 a comment that is edited elsewhere stays readable and has a way out", async ({
  page,
  browser,
  server,
}) => {
  await signIn(page, ada);
  await page.goto(issueAddress("WEB-1"));
  const box = page.getByRole("textbox", { name: t("comments.write"), exact: true });
  await box.fill("Seen it twice.");
  const composer = page.locator("form").filter({ has: box });
  await composer.getByRole("button", { name: t("comments.add"), exact: true }).click();
  await page.getByRole("tab", { name: t("history.tab-comments"), exact: true }).click();
  await pressEditUntil(
    page,
    page.getByRole("textbox", { name: t("comments.editing"), exact: true }),
  );

  const other = await anotherBrowser(browser, server);
  await signIn(other, ada);
  await other.goto(issueAddress("WEB-1"));
  await other.getByRole("tab", { name: t("history.tab-comments"), exact: true }).click();
  // The text stays, the note says why nothing can be typed, and there is a way out.
  await pressEditUntil(
    other,
    other.getByText(t("editing.elsewhere", { thing: t("editing.this-comment") })),
  );
  await expect(other.getByText("Seen it twice.")).toBeVisible();
  await expect(other.getByRole("button", { name: t("comments.edit"), exact: true })).toHaveCount(0);
  await other.getByRole("button", { name: t("common.close"), exact: true }).click();
  await expect(other.getByRole("button", { name: t("comments.edit"), exact: true })).toBeVisible();
  await expect(other.getByText("Seen it twice.")).toBeVisible();
});
