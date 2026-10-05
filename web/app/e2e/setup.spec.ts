import { t } from "@bugaboo/messages";
import { type Page } from "@playwright/test";

import { ada } from "./support/accounts.ts";
import { addVirtualAuthenticator } from "./support/authenticator.ts";
import { expectGoodInEveryLook } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { addPasskey } from "./support/flows.ts";

test.describe.configure({ mode: "serial" });

const wrongCode = "aaaa-bbbb-cccc-dddd";

/** Types the setup code on the first step and goes on to the account. */
async function enterCode(page: Page, code: string): Promise<void> {
  await page.getByLabel(t("first-run.code.label")).fill(code);
  await page.getByRole("button", { name: t("common.continue") }).click();
  await expect(page.getByRole("heading", { name: t("first-run.account.title") })).toBeVisible();
}

/** Fills in the account of the second step and sends it. */
async function sendAccount(page: Page, password: string): Promise<void> {
  await page.getByLabel(t("field.name")).fill(ada.name);
  await page.getByLabel(t("field.email")).fill(ada.email);
  await page.getByLabel(t("field.password"), { exact: true }).fill(password);
  await page.getByRole("button", { name: t("first-run.account.submit") }).click();
}

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
  await page.goto("/");
  await expect(page).toHaveURL(/\/setup$/u);
  await expect(page.getByRole("heading", { level: 1, name: t("first-run.title") })).toBeVisible();
  await expect(page.getByText(t("first-run.code.where"))).toBeVisible();
  await expect(
    page.getByText(t("first-run.step-of", { step: 1, total: 4, name: t("first-run.step.code") })),
  ).toBeVisible();
  await expectGoodInEveryLook(page);
});

test("auth_09 a wrong setup code is refused and the person goes back to the first step", async ({
  page,
}) => {
  await page.goto("/setup");
  await enterCode(page, wrongCode);
  await sendAccount(page, ada.password);
  await expect(page.getByRole("heading", { name: t("first-run.title") })).toBeVisible();
  await expect(page.getByRole("alert")).toHaveText(t("error.auth.wrong-setup-code"));
});

test("auth_01 a password that is too short is explained next to the field", async ({
  page,
  server,
}) => {
  await page.goto("/setup");
  await enterCode(page, server.setupCode());
  await sendAccount(page, "short");
  await expect(page.getByText(t("error.auth.password-too-short"))).toBeVisible();
  await expect(page.getByRole("heading", { name: t("first-run.account.title") })).toBeVisible();
});

test("auth_09 the setup code makes the first administrator, who can add a passkey", async ({
  page,
  server,
}) => {
  await addVirtualAuthenticator(page);
  await page.goto("/setup");
  await enterCode(page, server.setupCode());
  await expectGoodInEveryLook(page);
  await sendAccount(page, ada.password);
  await addTheFirstPasskey(page);

  await page.getByRole("button", { name: t("first-run.safer.finish") }).click();
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
  await page.goto("/setup");
  await expect(page).toHaveURL(/\/sign-in$/u);
  await expect(page.getByRole("heading", { level: 1, name: t("sign-in.title") })).toBeVisible();
});
