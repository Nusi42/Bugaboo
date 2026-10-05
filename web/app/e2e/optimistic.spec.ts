import { t } from "@bugaboo/messages";
import { type Browser, type Locator, type Page } from "@playwright/test";

import { ada, endEverySession, makeAdmin } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { type Instance } from "./support/server.ts";
import { anotherBrowser, openFromMenu, signIn } from "./support/flows.ts";

test.use({
  preparation: {
    run: async (server) => {
      await (await makeAdmin(server)).dispose();
    },
  },
});
test.describe.configure({ mode: "serial" });

/** The places where the person is signed in, as the page lists them. */
function places(page: Page): Locator {
  return page.getByRole("list", { name: t("sessions.list") }).getByRole("listitem");
}

/** The button that signs out of another place. */
function signOutOfOther(page: Page): Locator {
  const start = t("sessions.end-name", { device: "" }).trim();
  return page.getByRole("button", { name: new RegExp(`^${start}`, "u") });
}

/** Two places are signed in; this page shows them. */
async function twoPlaces(page: Page, browser: Browser, server: Instance): Promise<Page> {
  await endEverySession(server, ada);
  const other = await anotherBrowser(browser, server);
  await signIn(other, ada);
  await signIn(page, ada);
  await openFromMenu(page, t("nav.sessions"));
  await expect(places(page)).toHaveCount(2);
  return other;
}

test("ux_06 with a slow server the change shows at once and Saved follows", async ({
  page,
  browser,
  server,
}) => {
  const other = await twoPlaces(page, browser, server);
  const slow = Promise.withResolvers<void>();
  await page.route("**/api/v1/auth/sessions/*", async (route) => {
    await slow.promise;
    await route.continue();
  });

  await signOutOfOther(page).click();
  // The server has not answered, and the place is already gone from the list.
  await expect(places(page)).toHaveCount(1, { timeout: 1000 });
  await expect(page.getByRole("status").filter({ hasText: t("save.saving") })).toBeVisible();
  slow.resolve();
  await expect(page.getByRole("status").filter({ hasText: t("save.saved") })).toBeVisible();
  await expect(places(page)).toHaveCount(1);
  await other.context().close();
});

test("ux_06 a change the server refuses goes back with a plain message", async ({
  page,
  browser,
  server,
}) => {
  const other = await twoPlaces(page, browser, server);
  await page.route("**/api/v1/auth/sessions/*", async (route) => {
    await route.fulfill({
      status: 403,
      contentType: "application/problem+json",
      body: JSON.stringify({
        type: "about:blank",
        title: "Forbidden",
        status: 403,
        code: "access.forbidden",
        message_key: "error.access.forbidden",
      }),
    });
  });

  await signOutOfOther(page).click();
  await expect(
    page.getByRole("alert").filter({ hasText: t("error.access.forbidden") }),
  ).toBeVisible();
  await expect(page.getByRole("status").filter({ hasText: t("save.not-saved") })).toBeVisible();
  await expect(places(page)).toHaveCount(2);
  await other.context().close();
});
