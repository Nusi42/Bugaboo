import { t } from "@bugaboo/messages";
import { type Page } from "@playwright/test";

import { ana, signedInAdmin } from "./support/access.ts";
import { expectGoodInEveryLook } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { openFromMenu, signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";
import { eMailChangePath, type Mailbox, startMailbox, turnMailOn } from "./support/mailbox.ts";

// *My settings › Profile* (PRIV-06, ADR-0044, decision 9): the name changed in place with Undo, and
// a new e-mail address that counts once the link sent to it is opened.

let mailbox: Mailbox;
const newAddress = "ana.new@example.test";

test.use({
  preparation: {
    run: async (server) => {
      await prepareWeb(server);
      mailbox = await startMailbox();
      const admin = await signedInAdmin(server);
      await turnMailOn(admin, mailbox);
      await admin.dispose();
    },
  },
});
test.describe.configure({ mode: "serial" });

/** Opens the profile the way a person does: the menu, My settings, Profile. */
async function openProfile(page: Page): Promise<void> {
  await signIn(page, ana);
  await openFromMenu(page, t("nav.profile"));
  await expect(
    page.getByRole("heading", { level: 1, name: t("profile.title"), exact: true }),
  ).toBeVisible();
}

test("priv_06 the profile page changes the name with Undo", async ({ page }) => {
  await openProfile(page);
  await expectGoodInEveryLook(page);
  await page.getByRole("button", { name: t("profile.change-name", { name: ana.name }) }).click();
  const box = page.getByLabel(t("profile.name"), { exact: true });
  await box.fill("Ana Andersen");
  await box.press("Enter");
  await expect(page.getByRole("status").filter({ hasText: t("save.saved") })).toBeVisible();
  await expect(page.getByRole("button", { name: /Ana Andersen/u })).toBeVisible();
  await page.getByRole("button", { name: t("common.undo"), exact: true }).click();
  await expect(page.getByRole("button", { name: new RegExp(ana.name, "u") })).toBeVisible();
});

test("priv_06 a change of e-mail says to open the link", async ({ page }) => {
  await openProfile(page);
  await page.getByRole("button", { name: t("profile.change-e-mail"), exact: true }).click();
  await page.getByLabel(t("profile.new-e-mail"), { exact: true }).fill(newAddress);
  await page.getByLabel(t("field.confirm-password"), { exact: true }).fill(ana.password);
  await page.getByRole("button", { name: t("profile.send-link"), exact: true }).click();
  await expect(page.getByText(t("profile.link-sent", { address: newAddress }))).toBeVisible();
  await expectGoodInEveryLook(page);
  const mail = await mailbox.waitFor(newAddress);
  await page.goto(eMailChangePath(mail));
  await expect(page.getByText(t("profile.confirmed"))).toBeVisible();
  await expectGoodInEveryLook(page);
});
