import { settingKeys } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { type Page } from "@playwright/test";

import { bob, cleo, makeAdmin, makePerson } from "./support/accounts.ts";
import { expectGoodInEveryTheme } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { pressTwoStepSwitch, setOrganizationSwitch } from "./support/organization.ts";
import {
  addressEndingAt,
  addressWith,
  afterCodeApp,
  expectHome,
  sendPassword,
  setUpCodeApp,
  signIn,
  signOut,
} from "./support/flows.ts";
import { paths } from "../src/shared/paths.ts";

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
  await expect(page).toHaveURL(addressEndingAt(paths.setUpSecondStep));
  await expect(
    page.getByRole("heading", { level: 1, name: t("set-up-second-step.title") }),
  ).toBeVisible();
  await expect(page.getByText(t("set-up-second-step.why"))).toBeVisible();
  await expect(page.getByRole("region", { name: t("code-app.title") })).toBeVisible();
  // No sidebar, no menu: the person can do this one thing.
  await expect(page.getByRole("navigation", { name: t("shell.menu") })).toHaveCount(0);
}

test("auth_03 a person who is signed in is taken to set up a second step the moment the organization asks for one", async ({
  browser,
  page,
  server,
}) => {
  await signIn(page, bob);
  // An administrator presses the switch on Sign-in rules, in a browser of their own.
  await pressTwoStepSwitch(browser, server, true);
  // The next thing the page asks the server takes Bob there, in the same session (a page that
  // asks again, as when it is opened again, or as the projects of the sidebar do).
  await page.reload();
  await expectSetUpScreen(page);
  // Nothing else opens, not even by typing the address.
  await page.goto(paths.sessions);
  await expectSetUpScreen(page);
  await expect(page.getByText(t("set-up-second-step.how"))).toBeVisible();
  // A person who cannot or will not set it up can still leave.
  await expect(page.getByRole("button", { name: t("nav.sign-out"), exact: true })).toBeVisible();
  await expectGoodInEveryTheme(page);

  // The code app is set up here, with the password typed again, and then Bob goes on.
  await setUpCodeApp(page, bob.password, { ending: afterCodeApp.secondStepScreen });
  await expect(page.getByText(t("set-up-second-step.ready"))).toBeVisible();
  await expectGoodInEveryTheme(page);
  await page.getByRole("button", { name: t("set-up-second-step.continue") }).click();
  await expectHome(page, bob);

  // The screen is only for people who have to: now the address leads to the start page.
  await page.goto(paths.setUpSecondStep);
  await expectHome(page, bob);
  // And from now on signing in asks for the second step.
  await signOut(page);
  await sendPassword(page, bob);
  await expect(page).toHaveURL(addressWith(paths.secondStep));
  await setOrganizationSwitch(server, settingKeys.securityTwoStepRequired, false);
});

test("auth_03 a person who signs in with only a password and has to set up a second step can do only that", async ({
  page,
  server,
}) => {
  await setOrganizationSwitch(server, settingKeys.securityTwoStepRequired, true);
  await sendPassword(page, cleo);
  await expectSetUpScreen(page);
  // A person who cannot set it up now can leave, and finds the same screen when coming back.
  await page.getByRole("button", { name: t("nav.sign-out"), exact: true }).click();
  await expect(page).toHaveURL(addressWith(paths.signIn));
  await sendPassword(page, cleo);
  await expectSetUpScreen(page);
  await page.goto(paths.home);
  await expectSetUpScreen(page);
  await setUpCodeApp(page, cleo.password, { ending: afterCodeApp.secondStepScreen });
  await page.getByRole("button", { name: t("set-up-second-step.continue") }).click();
  await expectHome(page, cleo);
  await setOrganizationSwitch(server, settingKeys.securityTwoStepRequired, false);
  await signOut(page);
});
