import { t } from "@bugaboo/messages";
import { type Page } from "@playwright/test";

import { ada } from "./support/accounts.ts";
import { addVirtualAuthenticator } from "./support/authenticator.ts";
import { expectGoodInEveryLook } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import {
  addPasskey,
  addressEndingAt,
  enterSetupCode,
  sendSetupAccount,
  skipMailStep,
} from "./support/flows.ts";
import { paths } from "../src/shared/paths.ts";

test.describe.configure({ mode: "serial" });

const wrongCode = "aaaa-bbbb-cccc-dddd";

/** The third step: a passkey is added, as the first of the second checks for the account. */
async function addTheFirstPasskey(page: Page): Promise<void> {
  await expect(
    page.getByRole("heading", { level: 1, name: t("first-run.safer.title") }),
  ).toBeVisible();
  await addPasskey(page, "Test laptop", ada.password);
  await expect(page.getByRole("list", { name: t("passkeys.list") })).toContainText("Test laptop");
  await expectGoodInEveryLook(page);
}

test("auth_09 a fresh server opens the setup and says where the code is", async ({ page }) => {
  await page.goto(paths.home);
  await expect(page).toHaveURL(addressEndingAt(paths.setup));
  await expect(page.getByRole("heading", { level: 1, name: t("first-run.title") })).toBeVisible();
  await expect(page.getByText(t("first-run.code.where"))).toBeVisible();
  await expect(
    page.getByText(t("first-run.step-of", { step: 1, total: 5, name: t("first-run.step.code") })),
  ).toBeVisible();
  await expectGoodInEveryLook(page);
});

test("auth_09 a wrong setup code is refused and the person goes back to the first step", async ({
  page,
}) => {
  await page.goto(paths.setup);
  await enterSetupCode(page, wrongCode);
  await sendSetupAccount(page, ada);
  await expect(page.getByRole("heading", { name: t("first-run.title") })).toBeVisible();
  await expect(page.getByRole("alert")).toHaveText(t("error.auth.wrong-setup-code"));
});

test("auth_01 a password that is too short is explained next to the field", async ({
  page,
  server,
}) => {
  await page.goto(paths.setup);
  await enterSetupCode(page, server.setupCode());
  await sendSetupAccount(page, ada, "short");
  await expect(page.getByText(t("error.auth.password-too-short"))).toBeVisible();
  await expect(page.getByRole("heading", { name: t("first-run.account.title") })).toBeVisible();
});

test("auth_09 the setup code makes the first administrator, who can add a passkey", async ({
  page,
  server,
}) => {
  await addVirtualAuthenticator(page);
  await page.goto(paths.setup);
  await enterSetupCode(page, server.setupCode());
  await expectGoodInEveryLook(page);
  await sendSetupAccount(page, ada);
  await addTheFirstPasskey(page);

  await page.getByRole("button", { name: t("first-run.safer.finish") }).click();
  await expect(
    page.getByRole("heading", { level: 1, name: t("first-run.mail.title") }),
  ).toBeVisible();
  await expectGoodInEveryLook(page);
  await skipMailStep(page);
  await expect(
    page.getByRole("heading", { level: 1, name: t("first-run.done.title") }),
  ).toBeVisible();
  await expectGoodInEveryLook(page);
  await page.getByRole("button", { name: t("first-run.done.go") }).click();
  await expect(
    page.getByRole("heading", { level: 1, name: t("home.title", { name: ada.name }) }),
  ).toBeVisible();
  await expect(page.getByText(t("home.admin"))).toBeVisible();
});

test("auth_09 the setup is closed for good afterwards", async ({ page }) => {
  await page.goto(paths.setup);
  await expect(page).toHaveURL(addressEndingAt(paths.signIn));
  await expect(page.getByRole("heading", { level: 1, name: t("sign-in.title") })).toBeVisible();
});
