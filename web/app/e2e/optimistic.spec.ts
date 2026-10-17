import { t } from "@bugaboo/messages";
import { type Browser, type Locator, type Page, type Route } from "@playwright/test";

import { ada, endEverySession, createAdmin } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { type Instance } from "./support/server.ts";
import { anotherBrowser, openFromMenu, signIn } from "./support/flows.ts";
import { partOn } from "./support/styles.ts";
import { part } from "../../packages/ui/src/tokens.ts";
import { paths } from "../src/shared/paths.ts";

test.use({
  preparation: {
    run: async (server) => {
      await (await createAdmin(server)).dispose();
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

test("ux_06 after ending a place the focus goes to the next button, else the heading", async ({
  page,
  browser,
  server,
}) => {
  const other = await twoPlaces(page, browser, server);
  const third = await anotherBrowser(browser, server);
  await signIn(third, ada);
  await page.reload();
  await expect(places(page)).toHaveCount(3);
  await signOutOfOther(page).first().click();
  await expect(places(page)).toHaveCount(2);
  await expect(signOutOfOther(page)).toBeFocused();
  await signOutOfOther(page).click();
  await expect(places(page)).toHaveCount(1);
  await expect(page.getByRole("heading", { level: 1, name: t("sessions.title") })).toBeFocused();
  await other.context().close();
  await third.context().close();
});

test("ux_06 grey shapes stand in for a slow list, and the list says it is busy", async ({
  page,
  browser,
  server,
}) => {
  const other = await twoPlaces(page, browser, server);
  const slow = Promise.withResolvers<void>();
  await page.route("**/api/v1/auth/sessions", async (route) => {
    await slow.promise;
    await route.continue();
  });
  await page.goto(paths.sessions);
  const region = page.getByRole("list", { name: t("sessions.list") }).locator("..");
  await expect(region).toHaveAttribute("aria-busy", "true");
  // Nothing grey in the first 0.3 s, so a quick answer never flashes.
  const shapes = partOn(region, part.skeletonLine);
  await expect(shapes).toHaveCount(0);
  await expect(shapes.first()).toBeVisible({ timeout: 3000 });
  slow.resolve();
  await expect(places(page)).toHaveCount(2);
  await expect(region).toHaveAttribute("aria-busy", "false");
  await expect(shapes).toHaveCount(0);
  await other.context().close();
});

/** Answers the first request after a moment with a refusal and lets every other one through. */
function refuseTheFirst(): (route: Route) => Promise<void> {
  let sent = 0;
  return async (route) => {
    sent += 1;
    if (sent > 1) {
      await route.continue();
      return;
    }
    await new Promise((resolve) => {
      setTimeout(resolve, 600);
    });
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
  };
}

test("ux_06 a refusal stays on screen when a later change is accepted", async ({
  page,
  browser,
  server,
}) => {
  const other = await twoPlaces(page, browser, server);
  const third = await anotherBrowser(browser, server);
  await signIn(third, ada);
  await page.reload();
  await expect(places(page)).toHaveCount(3);
  // The first change is refused (after a moment), the second is accepted.
  await page.route("**/api/v1/auth/sessions/*", refuseTheFirst());
  await signOutOfOther(page).first().click();
  await signOutOfOther(page).first().click();
  // The accepted one is gone, the refused one is back, and its message is shown.
  await expect(places(page)).toHaveCount(2);
  await expect(
    page.getByRole("alert").filter({ hasText: t("error.access.forbidden") }),
  ).toBeVisible();
  const status = page.getByRole("status").filter({ hasText: t("save.not-saved") });
  await expect(status).toBeVisible();
  await expect(page.getByText(t("save.saved"), { exact: true })).toHaveCount(0);
  await other.context().close();
  await third.context().close();
});
