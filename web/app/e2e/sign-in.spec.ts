import { t } from "@bugaboo/messages";

import { ada, makeAdmin } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { expectHome, sendPassword, signIn, signOut } from "./support/flows.ts";

test.use({
  preparation: {
    run: async (server) => {
      await (await makeAdmin(server)).dispose();
    },
  },
});
test.describe.configure({ mode: "serial" });

test("auth_01 somebody who is not signed in is taken to the sign-in page", async ({ page }) => {
  await page.goto("/");
  await expect(page).toHaveURL(/\/sign-in$/u);
  await expect(page.getByRole("heading", { level: 1, name: t("sign-in.title") })).toBeVisible();
  await page.goto("/account/security");
  await expect(page).toHaveURL(/\/sign-in$/u);
});

test("auth_01 the right password opens the start page, and a reload keeps the person signed in", async ({
  page,
}) => {
  await signIn(page, ada);
  await expect(page).toHaveURL(/\/$/u);
  await page.reload();
  await expectHome(page, ada);
  await expect(page.getByText(t("shell.signed-in-as", { name: ada.name }))).toBeVisible();
});

test("auth_01 a wrong password and an unknown address get the same answer", async ({ page }) => {
  await sendPassword(page, ada, "not-the-password");
  await expect(page.getByRole("alert")).toHaveText(t("error.auth.wrong-credentials"));
  await sendPassword(page, { ...ada, email: "nobody@example.test" });
  await expect(page.getByRole("alert")).toHaveText(t("error.auth.wrong-credentials"));
  await expect(page).toHaveURL(/\/sign-in$/u);
});

test("auth_01 empty fields are explained before anything is sent", async ({ page }) => {
  await page.goto("/sign-in");
  await page.getByRole("button", { name: t("sign-in.submit"), exact: true }).click();
  await expect(page.getByText(t("validation.required")).first()).toBeVisible();
  await page.getByLabel(t("field.email")).fill("no-at-sign");
  await page.getByLabel(t("field.password"), { exact: true }).fill("x");
  await page.getByRole("button", { name: t("sign-in.submit"), exact: true }).click();
  await expect(page.getByText(t("validation.email"))).toBeVisible();
});

test("auth_01 signing out ends the sign-in, and the pages need a new one", async ({ page }) => {
  await signIn(page, ada);
  await signOut(page);
  await expect(page.getByText(t("sign-in.signed-out"))).toBeVisible();
  await page.goto("/account/sessions");
  await expect(page).toHaveURL(/\/sign-in$/u);
  await sendPassword(page, ada);
  await expectHome(page, ada);
});

test("auth_01 a person who is signed in does not see the sign-in page again", async ({ page }) => {
  await signIn(page, ada);
  await page.goto("/sign-in");
  await expect(page).toHaveURL(/\/$/u);
});
