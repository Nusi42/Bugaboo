import { CSRF_COOKIE } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";

import { ada, makeAdmin } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import {
  anotherBrowser,
  expectHome,
  fillSignIn,
  openFromMenu,
  pressSignOut,
  signIn,
  signOut,
} from "./support/flows.ts";

test.use({
  preparation: {
    run: async (server) => {
      await (await makeAdmin(server)).dispose();
    },
  },
});

test("m1_f51 an answer that says the token is wrong takes the person to the sign-in page", async ({
  page,
  context,
  server,
}) => {
  await signIn(page, ada);
  await openFromMenu(page, t("nav.security"));
  // The token of the session is somebody else's now, as after the server lost its CSRF key.
  await context.addCookies([
    { name: CSRF_COOKIE, value: "not-the-token", url: server.url, secure: true },
  ]);
  await pressSignOut(page);
  await expect(page).toHaveURL(/\/sign-in\?reason=wrong-token$/u);
  await expect(page.getByRole("heading", { level: 1, name: t("sign-in.title") })).toBeVisible();
  await expect(page.getByText(t("error.request.wrong-token"))).toBeVisible();
  // Signing in again from this page gives a session whose token fits, and everything works again.
  await fillSignIn(page, ada);
  await expectHome(page, ada);
  await signOut(page);
});

test("m1_f51 an answer that says nobody is signed in takes the person to the sign-in page", async ({
  page,
  browser,
  server,
}) => {
  await signIn(page, ada);
  // The same person signs out everywhere from another browser.
  const other = await anotherBrowser(browser, server);
  await signIn(other, ada);
  await openFromMenu(other, t("nav.sessions"));
  await other.getByRole("button", { name: t("sessions.everywhere") }).click();
  await other
    .getByRole("alertdialog")
    .getByRole("button", { name: t("sessions.everywhere") })
    .click();
  await expect(other).toHaveURL(/\/sign-in/u);
  await other.context().close();

  await openFromMenu(page, t("nav.sessions"));
  await expect(page).toHaveURL(/\/sign-in\?reason=session-ended$/u);
  await expect(page.getByText(t("error.auth.not-signed-in"))).toBeVisible();
});
