import { t } from "@bugaboo/messages";
import { type Page } from "@playwright/test";

import { ada } from "./support/accounts.ts";
import { addVirtualAuthenticator } from "./support/authenticator.ts";
import { expect, test } from "./support/fixtures.ts";
import {
  addPasskey,
  addressEndingAt,
  enterSetupCode,
  expectHome,
  fillSignIn,
  openFromMenu,
  sendSecondStepCode,
  sendSetupAccount,
  setUpCodeApp,
  signOut,
} from "./support/flows.ts";
import { paths } from "../src/shared/paths.ts";

// The exit demo of M1 (docs/04-milestones.md): set up with the setup code, make the administrator
// with a passkey, add the code app, sign out, sign in with password and code. The security log
// shows every step, which the status page and the program's own check both confirm.

test.describe.configure({ mode: "serial" });

/** What every step wrote into the security log, in the order of the steps (`security-log.*`). */
const entriesAfter = {
  /** `starting_roles.made`, `setup.completed`, `assignment.made` (the Administrator role), then `passkey_added`. */
  setupWithPasskey: 4,
  /** `two_step_enabled`. */
  codeApp: 5,
  /** `session_ended` (sign out), `sign_in_second_step_needed`, `second_step_succeeded`. */
  signedInAgain: 8,
} as const;

/** Opens the status page and expects the security log to be intact with exactly `entries` entries. */
async function expectLogIntact(page: Page, entries: number): Promise<void> {
  await openFromMenu(page, t("nav.status"));
  await expect(
    page.getByRole("heading", { level: 1, name: t("server-status.title") }),
  ).toBeVisible();
  await expect(page.getByText(t("security-log.valid", { entries }))).toBeVisible();
  await expect(page.getByText(t("security-log.unanchored"))).toHaveCount(0);
}

test("the exit demo: setup, passkey, code app, sign out, sign in, and the log shows each step", async ({
  page,
  server,
}) => {
  await addVirtualAuthenticator(page);

  // 1. The setup code makes the first administrator, who adds a passkey in the same setup.
  await page.goto(paths.home);
  await expect(page).toHaveURL(addressEndingAt(paths.setup));
  await enterSetupCode(page, server.setupCode());
  await sendSetupAccount(page, ada);
  await addPasskey(page, "Demo laptop", ada.password);
  await page.getByRole("button", { name: t("first-run.safer.finish") }).click();
  await page.getByRole("button", { name: t("first-run.done.go") }).click();
  await expectHome(page, ada);
  await expectLogIntact(page, entriesAfter.setupWithPasskey);

  // 2. The code app is added on the security page.
  await openFromMenu(page, t("nav.security"));
  const { app } = await setUpCodeApp(page, ada.password);
  await expectLogIntact(page, entriesAfter.codeApp);

  // 3. Signing out and in again needs the password and a code.
  await signOut(page);
  await fillSignIn(page, ada);
  await expect(page.getByRole("heading", { level: 1, name: t("second-step.title") })).toBeVisible();
  await sendSecondStepCode(page, await app.next());
  await expectHome(page, ada);
  await expectLogIntact(page, entriesAfter.signedInAgain);

  // 4. The program's own check of the data folder agrees with the status page.
  const verdict = await server.verifySecurityLog();
  expect(verdict).toContain(t("security-log.valid", { entries: entriesAfter.signedInAgain }));
});
