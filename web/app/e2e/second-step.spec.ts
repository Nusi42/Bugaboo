import { t } from "@bugaboo/messages";
import { type Page } from "@playwright/test";

import { addCodeApp, bob, createAdmin, createPerson, type TwoStep } from "./support/accounts.ts";
import { addVirtualAuthenticator } from "./support/authenticator.ts";
import { expectGoodInEveryTheme } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import {
  addPasskey,
  addressEndingAt,
  addressWith,
  expectHome,
  openFromMenu,
  sendPassword,
  sendSecondStepCode,
  signOut,
} from "./support/flows.ts";
import { paths } from "../src/shared/paths.ts";

/** Bob has a code app, set up through the API before the first test. */
let twoStep: TwoStep;

test.use({
  preparation: {
    run: async (server) => {
      const admin = await createAdmin(server);
      const session = await createPerson(server, admin, bob);
      twoStep = await addCodeApp(session, bob.password);
      await admin.dispose();
      await session.dispose();
    },
  },
});
test.describe.configure({ mode: "serial" });

/** Signs in with the password and a code from the code app. */
async function signInWithCode(page: Page): Promise<void> {
  await sendPassword(page, bob);
  await sendSecondStepCode(page, await twoStep.app.next());
  await expectHome(page, bob);
}

test("auth_03 the password alone does not open the account", async ({ page }) => {
  await sendPassword(page, bob);
  await expect(page).toHaveURL(addressWith(paths.secondStep));
  await expect(page.getByRole("heading", { level: 1, name: t("second-step.title") })).toBeVisible();
  await expect(page.getByText(t("second-step.code-help"))).toBeVisible();
  await expect(page.getByRole("button", { name: t("second-step.passkey") })).toHaveCount(0);
  // Nothing of the account shows before the second step is done.
  await page.goto(paths.home);
  await expect(page).toHaveURL(addressEndingAt(paths.signIn));
});

test("auth_03 a wrong code is explained and the person can try again", async ({ page }) => {
  await sendPassword(page, bob);
  await sendSecondStepCode(page, "000000");
  await expect(page.getByText(t("error.auth.wrong-second-step-code"))).toBeVisible();
  await expect(page).toHaveURL(addressWith(paths.secondStep));
  await sendSecondStepCode(page, await twoStep.app.next());
  await expectHome(page, bob);
});

test("auth_03 a code from the code app finishes signing in", async ({ page }) => {
  await signInWithCode(page);
  await signOut(page);
});

test("auth_03 a backup code works once", async ({ page }) => {
  const backup = String(twoStep.backupCodes.at(0));
  await sendPassword(page, bob);
  await sendSecondStepCode(page, backup);
  await expectHome(page, bob);
  await signOut(page);

  await sendPassword(page, bob);
  await sendSecondStepCode(page, backup);
  await expect(page.getByText(t("error.auth.wrong-second-step-code"))).toBeVisible();
});

test("auth_02 a passkey is added, and then serves as the second step and as the whole sign-in", async ({
  page,
}) => {
  await addVirtualAuthenticator(page);
  await signInWithCode(page);
  await openFromMenu(page, t("nav.security"));
  await addPasskey(page, "Bob phone", bob.password);
  await signOut(page);

  // After the password, the passkey is offered next to the code app.
  await sendPassword(page, bob);
  await expect(page.getByRole("button", { name: t("second-step.passkey") })).toBeVisible();
  await expect(page.getByLabel(t("second-step.code"))).toBeVisible();
  await expectGoodInEveryTheme(page);
  await page.getByRole("button", { name: t("second-step.passkey") }).click();
  await expectHome(page, bob);
  await signOut(page);

  // And a passkey signs in without a password.
  await page.getByRole("button", { name: t("sign-in.passkey") }).click();
  await expectHome(page, bob);
});
