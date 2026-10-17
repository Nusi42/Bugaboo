import { handleChangeEveryDays, handleMax } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { type Page } from "@playwright/test";

import { ana } from "./support/access.ts";
import { bob } from "./support/accounts.ts";
import { expectGoodInEveryLook } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { anotherBrowser, openFromMenu, signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";

// The handle in *My settings* (M3-F70, M4-Q9, M4 WP-08): shown, changed, and the rules said in
// plain words: ASCII only, once in seven days, a former handle stays reserved, never the handle of
// a group or a role.

test.use({ preparation: { run: prepareWeb } });
test.describe.configure({ mode: "serial" });

/** Opens the page of the handle the way a person does: the menu, My settings, Handle. */
async function openHandle(page: Page): Promise<void> {
  await signIn(page, ana);
  await openFromMenu(page, t("nav.handle"));
  await expect(
    page.getByRole("heading", { level: 1, name: t("handle.title"), exact: true }),
  ).toBeVisible();
}

/** Types a handle and presses *Change my handle*. */
async function changeTo(page: Page, handle: string): Promise<void> {
  await page.getByLabel(t("handle.label"), { exact: true }).fill(handle);
  await page.getByRole("button", { name: t("handle.save"), exact: true }).click();
}

test("ntf_01 the handle is shown with the rules in plain words", async ({ page }) => {
  await openHandle(page);
  await expect(page.getByLabel(t("handle.label"), { exact: true })).toHaveValue("ana.analyst");
  await expect(page.getByText(t("handle.rule-characters", { max: handleMax }))).toBeVisible();
  await expect(page.getByText(t("handle.rule-own"))).toBeVisible();
  await expect(
    page.getByText(t("handle.rule-weekly", { days: handleChangeEveryDays })),
  ).toBeVisible();
  await expect(page.getByText(t("handle.rule-kept"))).toBeVisible();
  await expectGoodInEveryLook(page);
});

test("ntf_01 a handle with accents, or one somebody has, is refused in words", async ({ page }) => {
  await openHandle(page);
  await changeTo(page, "änä");
  await expect(page.getByRole("alert")).toHaveText(t("handle.invalid"));
  await changeTo(page, "bob.builder");
  await expect(page.getByRole("alert")).toHaveText(t("handle.taken"));
});

test("ntf_01 a handle can be changed once in a while, and the old one stays reserved", async ({
  page,
  browser,
  server,
}) => {
  await openHandle(page);
  await changeTo(page, "ana.a");
  await expect(page.getByText(t("handle.changed", { handle: "ana.a" }))).toBeVisible();
  await changeTo(page, "ana.b");
  await expect(page.getByRole("alert")).toHaveText(t("error.handle.changed-lately"));

  // The handle Ana gave up is hers: nobody else can take it.
  const ben = await anotherBrowser(browser, server);
  await signIn(ben, bob);
  await openFromMenu(ben, t("nav.handle"));
  await changeTo(ben, "ana.analyst");
  await expect(ben.getByRole("alert")).toHaveText(t("handle.taken"));
});
