import { t } from "@bugaboo/messages";
import { type Page } from "@playwright/test";

import { ada, addCodeApp, bob, cleo, invite, makeAdmin, makePerson } from "./support/accounts.ts";
import { expectGoodInEveryLook } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { openFromMenu, sendPassword, signIn } from "./support/flows.ts";
import { codeApp } from "./support/totp.ts";

/** The link of an invitation that nobody has used. */
let openLink = "";

test.use({
  preparation: {
    run: async (server) => {
      const admin = await makeAdmin(server);
      const session = await makePerson(server, admin, bob);
      await addCodeApp(session, bob.password);
      await (await makePerson(server, admin, cleo)).dispose();
      openLink = await invite(admin, "guest@example.test");
      await admin.put("/api/v1/auth/sign-up-policy", { enabled: true, domains: ["example.test"] });
      await admin.dispose();
      await session.dispose();
    },
  },
});

/** Opens an address, waits for its heading, and checks the screen in every theme. */
async function checkScreen(page: Page, address: string, heading: string): Promise<void> {
  await page.goto(address);
  await expect(page.getByRole("heading", { level: 1, name: heading })).toBeVisible();
  await expectGoodInEveryLook(page);
}

/** Sets up the code app on the security page, checking every stage on the way. */
async function setUpCodeApp(page: Page): Promise<void> {
  const section = page.getByRole("region", { name: t("code-app.title") });
  await section.getByLabel(t("field.confirm-password"), { exact: true }).fill(cleo.password);
  await section.getByRole("button", { name: t("code-app.setup") }).click();
  await expect(page.getByRole("img", { name: t("code-app.qr-alt") })).toBeVisible();
  await expectGoodInEveryLook(page);

  const secret = (await section.locator("code").textContent()) ?? "";
  await section.getByLabel(t("code-app.confirm-code")).fill(await codeApp(secret).next());
  await section.getByRole("button", { name: t("code-app.confirm") }).click();
  await expect(page.getByText(t("backup.shown-once"))).toBeVisible();
  await expectGoodInEveryLook(page);
  await page.getByRole("button", { name: t("backup.saved") }).click();
  await expect(page.getByRole("region", { name: t("backup.title") })).toBeVisible();
}

test("ux_04 sign in, sign up, the invitation and a page that is not there", async ({ page }) => {
  await checkScreen(page, "/sign-in", t("sign-in.title"));
  await expect(page.getByRole("button", { name: t("sign-in.passkey") })).toBeVisible();

  // A sign-in that failed shows its message in place.
  await page.getByLabel(t("field.email")).fill(ada.email);
  await page.getByLabel(t("field.password"), { exact: true }).fill("not-the-password");
  await page.getByRole("button", { name: t("sign-in.submit"), exact: true }).click();
  await expect(page.getByRole("alert")).toBeVisible();
  await expectGoodInEveryLook(page);

  await checkScreen(page, "/sign-up", t("sign-up.title"));
  await checkScreen(page, openLink, t("invitation.title"));
  await checkScreen(page, "/invitation#not-a-secret", t("invitation.invalid-title"));
  await checkScreen(page, "/there-is-nothing-here", t("page.not-found.title"));
});

test("ux_04 the second step", async ({ page }) => {
  await sendPassword(page, bob);
  await expect(page.getByRole("heading", { level: 1, name: t("second-step.title") })).toBeVisible();
  await expectGoodInEveryLook(page);
});

test("ux_04 home, sign-in and security with the code app, and the sessions", async ({ page }) => {
  await signIn(page, cleo);
  await expectGoodInEveryLook(page);

  await openFromMenu(page, t("nav.security"));
  await expect(page.getByRole("heading", { level: 1, name: t("security.title") })).toBeVisible();
  await expectGoodInEveryLook(page);
  await setUpCodeApp(page);
  await expectGoodInEveryLook(page);

  await openFromMenu(page, t("nav.sessions"));
  await expect(page.getByRole("heading", { level: 1, name: t("sessions.title") })).toBeVisible();
  await expectGoodInEveryLook(page);
  await page.getByRole("button", { name: t("sessions.everywhere") }).click();
  await expect(page.getByRole("alertdialog")).toBeVisible();
  await expectGoodInEveryLook(page);
});

test("ux_04 the invitations with the questions that open over them", async ({ page }) => {
  await signIn(page, ada);
  await openFromMenu(page, t("nav.people"));
  await page.getByLabel(t("field.email"), { exact: true }).fill("new@example.test");
  await page.getByRole("button", { name: t("invitations.create") }).click();
  await expect(page.getByText(t("invitations.link-help"))).toBeVisible();
  await expectGoodInEveryLook(page);

  const policy = page.getByRole("region", { name: t("sign-up-policy.title") });
  await policy.getByLabel(t("sign-up-policy.domains")).fill("");
  await expect(policy.getByText(t("sign-up-policy.warning"))).toBeVisible();
  await expectGoodInEveryLook(page);
  await policy.getByRole("button", { name: t("common.save") }).click();
  await expect(page.getByRole("alertdialog")).toBeVisible();
  await expectGoodInEveryLook(page);
  await page.getByRole("button", { name: t("sign-up-policy.confirm-no") }).click();

  await page
    .getByRole("button", { name: t("invitations.take-back-name", { email: "new@example.test" }) })
    .click();
  await expect(page.getByRole("alertdialog")).toBeVisible();
  await expectGoodInEveryLook(page);
});

test("ux_04 the status of the server", async ({ page }) => {
  await signIn(page, ada);
  await openFromMenu(page, t("nav.status"));
  await expect(page.getByRole("heading", { level: 1, name: t("status.title") })).toBeVisible();
  await expect(page.getByText(t("status.database.ok"))).toBeVisible();
  await expectGoodInEveryLook(page);
});
