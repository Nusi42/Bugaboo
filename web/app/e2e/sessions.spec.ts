import { t } from "@bugaboo/messages";
import { type Locator, type Page } from "@playwright/test";

import { ada, endEverySession, makeAdmin } from "./support/accounts.ts";
import { expectGoodInEveryTheme } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { anotherBrowser, openFromMenu, signIn } from "./support/flows.ts";

test.use({
  preparation: {
    run: async (server) => {
      await (await makeAdmin(server)).dispose();
    },
  },
});
test.describe.configure({ mode: "serial" });

/** The button that signs out of another place: its name starts with the same words whatever the place. */
function signOutOfOther(page: Page): Locator {
  const start = t("sessions.end-name", { device: "" }).trim();
  return page.getByRole("button", { name: new RegExp(`^${start}`, "u") });
}

/** The places where the person is signed in, as the page lists them. */
function places(page: Page): Locator {
  return page.getByRole("list", { name: t("sessions.list") }).getByRole("listitem");
}

/** Whether the page is the sign-in page. */
async function expectSignInPage(page: Page): Promise<void> {
  await expect(page.getByRole("heading", { level: 1, name: t("sign-in.title") })).toBeVisible();
}

test("sec_12 every browser is listed and this one is marked", async ({ page, browser, server }) => {
  await endEverySession(server, ada);
  const other = await anotherBrowser(browser, server);
  await signIn(other, ada);
  await signIn(page, ada);

  await openFromMenu(page, t("nav.sessions"));
  await expect(page.getByRole("heading", { level: 1, name: t("sessions.title") })).toBeVisible();
  await expect(places(page)).toHaveCount(2);
  await expect(page.getByText(t("sessions.this"))).toHaveCount(1);
  // The other place has a button to sign out of it; this one has its own in the menu.
  await expect(signOutOfOther(page)).toHaveCount(1);
  // The button shows "Sign out" and its name says which place (label in name, WCAG 2.5.3).
  await expect(signOutOfOther(page)).toHaveText(t("sessions.end"));
  await expectGoodInEveryTheme(page);
  await other.context().close();
});

test("sec_12 a place can be signed out of from another one", async ({ page, browser, server }) => {
  await endEverySession(server, ada);
  const other = await anotherBrowser(browser, server);
  await signIn(other, ada);
  await signIn(page, ada);
  await openFromMenu(page, t("nav.sessions"));
  await expect(places(page)).toHaveCount(2);

  await signOutOfOther(page).click();
  await expect(places(page)).toHaveCount(1);
  // The other browser finds out when it next asks something, and is taken to sign in.
  await other.reload();
  await expectSignInPage(other);
  await other.context().close();
});

test("auth_01 signing out everywhere ends every place, this one included", async ({
  page,
  browser,
  server,
}) => {
  await endEverySession(server, ada);
  const other = await anotherBrowser(browser, server);
  await signIn(other, ada);
  await signIn(page, ada);

  await openFromMenu(page, t("nav.sessions"));
  await page.getByRole("button", { name: t("sessions.everywhere") }).click();
  await expect(page.getByRole("alertdialog")).toContainText(t("sessions.everywhere-text"));
  await page
    .getByRole("alertdialog")
    .getByRole("button", { name: t("sessions.everywhere") })
    .click();
  await expect(page).toHaveURL(/\/sign-in/u);
  await expect(page.getByText(t("sign-in.signed-out"))).toBeVisible();

  await other.reload();
  await expectSignInPage(other);
  await other.context().close();
});
