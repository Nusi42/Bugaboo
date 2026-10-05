import { t } from "@bugaboo/messages";
import { type Page } from "@playwright/test";

import { bob, cleo, makeAdmin, makePerson } from "./support/accounts.ts";
import { expectGoodInEveryTheme } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import {
  expectHome,
  openFromMenu,
  sendPassword,
  setUpCodeApp,
  signIn,
  signOut,
} from "./support/flows.ts";

/** The setting that asks everybody in the organization for a second step. */
const REQUIRED = "security.two-step-required";

test.use({
  preparation: {
    run: async (server) => {
      const admin = await makeAdmin(server);
      await (await makePerson(server, admin, bob)).dispose();
      await (await makePerson(server, admin, cleo)).dispose();
      await admin.dispose();
    },
  },
});
test.describe.configure({ mode: "serial" });

/** The screen that asks for a second step is open, and the app shows nothing else. */
async function expectSetUpScreen(page: Page): Promise<void> {
  await expect(page).toHaveURL(/\/sign-in\/set-up-second-step$/u);
  await expect(
    page.getByRole("heading", { level: 1, name: t("set-up-second-step.title") }),
  ).toBeVisible();
  await expect(page.getByText(t("set-up-second-step.why"))).toBeVisible();
  await expect(page.getByRole("region", { name: t("code-app.title") })).toBeVisible();
  // No sidebar, no menu: the person can do this one thing.
  await expect(page.getByRole("navigation", { name: t("shell.menu") })).toHaveCount(0);
}

test("auth_03 a person who is signed in is taken to set up a second step the moment the organization asks for one", async ({
  page,
  server,
}) => {
  await signIn(page, bob);
  server.setOrganizationSwitch(REQUIRED, true);
  // The next thing the page asks the server takes Bob there, in the same session.
  await openFromMenu(page, t("nav.sessions"));
  await expectSetUpScreen(page);
  // Nothing else opens, not even by typing the address.
  await page.goto("/me/sessions");
  await expectSetUpScreen(page);
  await expect(page.getByText(t("set-up-second-step.waiting"))).toBeVisible();
  await expectGoodInEveryTheme(page);

  // The code app is set up here, with the password typed again, and then Bob goes on.
  await setUpCodeApp(page, bob.password, false);
  await expect(page.getByText(t("set-up-second-step.ready"))).toBeVisible();
  await expectGoodInEveryTheme(page);
  await page.getByRole("button", { name: t("set-up-second-step.continue") }).click();
  await expectHome(page, bob);

  // The screen is only for people who have to: now the address leads to the start page.
  await page.goto("/sign-in/set-up-second-step");
  await expectHome(page, bob);
  // And from now on signing in asks for the second step.
  await signOut(page);
  await sendPassword(page, bob);
  await expect(page).toHaveURL(/\/sign-in\/second-step/u);
  server.setOrganizationSwitch(REQUIRED, false);
});

test("auth_03 a person who signs in with only a password and has to set up a second step can do only that", async ({
  page,
  server,
}) => {
  server.setOrganizationSwitch(REQUIRED, true);
  await sendPassword(page, cleo);
  await expectSetUpScreen(page);
  await page.goto("/");
  await expectSetUpScreen(page);
  await setUpCodeApp(page, cleo.password, false);
  await page.getByRole("button", { name: t("set-up-second-step.continue") }).click();
  await expectHome(page, cleo);
  server.setOrganizationSwitch(REQUIRED, false);
  await signOut(page);
});
