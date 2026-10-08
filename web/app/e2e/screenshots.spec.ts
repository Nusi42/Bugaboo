import { t } from "@bugaboo/messages";
import { type Browser, type Page } from "@playwright/test";

import { ada, setSignUpPolicy } from "./support/accounts.ts";
import { addVirtualAuthenticator } from "./support/authenticator.ts";
import { expect, test } from "./support/fixtures.ts";
import {
  addPasskey,
  afterCodeApp,
  anotherBrowser,
  apiOf,
  enterSetupCode,
  expectHome,
  fillSignIn,
  openFromMenu,
  sendPassword,
  sendSecondStepCode,
  sendSetupAccount,
  setUpCodeApp,
  signOut,
} from "./support/flows.ts";
import { type Instance } from "./support/server.ts";
import { screenshotFolder, shoot } from "./support/screenshots.ts";
import { paths } from "../src/shared/paths.ts";
import { viewports } from "../../tooling/viewports.ts";

// The screens of the milestone for the maintainer's review, at phone width, one light and one dark
// theme each. They are only taken when `BB_SCREENSHOT_DIR` names the folder they go to:
//   BB_SCREENSHOT_DIR=... npx playwright test screenshots.spec --project=phone
test.skip(screenshotFolder === undefined, "Set BB_SCREENSHOT_DIR to take the screenshots.");
/** All the screens, each in two looks, take longer than an ordinary test. */
const TAKING_ALL_MS = 300_000;
test.use({ viewport: viewports.phoneForPictures, deviceScaleFactor: 2 });

/** The first two steps of the setup, with a screenshot of each. */
async function setUpTheFirstAdministrator(page: Page, setupCode: string): Promise<void> {
  await page.goto(paths.setup);
  await expect(page.getByRole("heading", { level: 1, name: t("first-run.title") })).toBeVisible();
  await shoot(page, "01-setup");
  await enterSetupCode(page, setupCode);
  await shoot(page, "02-setup-account");
  await sendSetupAccount(page, ada);
  await expect(page.getByRole("heading", { name: t("first-run.safer.title") })).toBeVisible();
}

/** The third step: a passkey and the code app. Returns the first backup code, for a later sign-in. */
async function keepTheAccountSafe(page: Page): Promise<string> {
  await addPasskey(page, "My phone", ada.password);
  await shoot(page, "03-setup-safety");
  const { backupCodes } = await setUpCodeApp(page, ada.password, {
    onPicture: () => shoot(page, "04-setup-code-app"),
    onBackupCodes: () => shoot(page, "05-setup-backup-codes"),
    ending: afterCodeApp.secondStepScreen,
  });
  return backupCodes[0] ?? "";
}

/** The end of the setup, and the start page and the security page. */
async function finishTheSetup(page: Page): Promise<void> {
  await page.getByRole("button", { name: t("first-run.safer.finish") }).click();
  await expect(page.getByRole("heading", { name: t("first-run.done.title") })).toBeVisible();
  await shoot(page, "06-setup-done");
  await page.getByRole("button", { name: t("first-run.done.go") }).click();
  await expectHome(page, ada);
  await shoot(page, "09-home");
  await openFromMenu(page, t("nav.security"));
  await expect(page.getByRole("heading", { level: 1, name: t("security.title") })).toBeVisible();
  await shoot(page, "10-security");
}

/** A second browser of the administrator, signed in with a backup code, so there is a second place. */
async function secondPlace(browser: Browser, server: Instance, backupCode: string): Promise<Page> {
  const other = await anotherBrowser(browser, server);
  await sendPassword(other, ada);
  await sendSecondStepCode(other, backupCode);
  await expectHome(other, ada);
  return other;
}

/** Lets people sign up on their own, from the browser of the signed-in administrator. */
async function switchSignUpOn(page: Page, server: Instance): Promise<void> {
  await setSignUpPolicy(await apiOf(page, server), { enabled: true, domains: ["example.test"] });
}

/** The pages of an administrator: sessions, invitations (and the link), status. Returns the link. */
async function theAdministratorsPages(page: Page): Promise<string> {
  await openFromMenu(page, t("nav.sessions"));
  await expect(page.getByRole("heading", { level: 1, name: t("sessions.title") })).toBeVisible();
  await expect(
    page.getByRole("list", { name: t("sessions.list") }).getByRole("listitem"),
  ).toHaveCount(2);
  await shoot(page, "11-where-signed-in");
  await openFromMenu(page, t("nav.people"));
  await page.getByLabel(t("field.email")).fill("new.colleague@example.test");
  await page.getByRole("button", { name: t("invitations.create") }).click();
  await expect(page.getByText(t("invitations.link-help"))).toBeVisible();
  await shoot(page, "12-invitations");
  const link = (await page.getByLabel(t("invitations.link")).textContent()) ?? "";
  await openFromMenu(page, t("nav.status"));
  await expect(page.getByText(t("status.database.ok"))).toBeVisible();
  await shoot(page, "15-status");
  return link;
}

/** What somebody sees who is not signed in: the invitation and the sign-up page. */
async function theGuestsPages(guest: Page, link: string): Promise<void> {
  await guest.goto(link);
  await expect(guest.getByRole("heading", { level: 1, name: t("invitation.title") })).toBeVisible();
  await shoot(guest, "13-invitation-link");
  await guest.goto(paths.signUp);
  await expect(guest.getByRole("heading", { level: 1, name: t("sign-up.title") })).toBeVisible();
  await shoot(guest, "14-sign-up");
}

test("the screens of the milestone", async ({ page, browser, server }) => {
  test.setTimeout(TAKING_ALL_MS);
  await addVirtualAuthenticator(page);
  await setUpTheFirstAdministrator(page, server.setupCode());
  const backupCode = await keepTheAccountSafe(page);
  await finishTheSetup(page);

  await switchSignUpOn(page, server);
  const other = await secondPlace(browser, server, backupCode);
  const link = await theAdministratorsPages(page);
  await other.context().close();

  const guest = await anotherBrowser(browser, server);
  await theGuestsPages(guest, link);
  await guest.context().close();

  await signOut(page);
  await shoot(page, "07-sign-in");
  await fillSignIn(page, ada);
  await expect(page.getByRole("heading", { level: 1, name: t("second-step.title") })).toBeVisible();
  await shoot(page, "08-second-step");
});
