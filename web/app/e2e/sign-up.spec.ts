import { t } from "@bugaboo/messages";
import { type Locator, type Page } from "@playwright/test";

import { ada, bob, makeAdmin } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { expectHome, openFromMenu, signIn, signOut } from "./support/flows.ts";

test.use({
  preparation: {
    run: async (server) => {
      await (await makeAdmin(server)).dispose();
    },
  },
});
test.describe.configure({ mode: "serial" });

/** Flips the switch of the sign-up rule with a click on its words, as a person does, and checks its state. */
async function setSwitch(policy: Locator, on: boolean): Promise<void> {
  await policy.getByText(t("sign-up-policy.switch")).click();
  await expect(policy.getByRole("switch", { name: t("sign-up-policy.switch") })).toBeChecked({
    checked: on,
  });
}

/** The part of the invitations page where an administrator decides who may sign up alone. */
async function openPolicy(page: Page): Promise<Locator> {
  await signIn(page, ada);
  await openFromMenu(page, t("nav.invitations"));
  return page.getByRole("region", { name: t("sign-up-policy.title") });
}

/** Fills in the sign-up page and sends it. */
async function sendSignUp(
  page: Page,
  name: string,
  email: string,
  password: string,
): Promise<void> {
  await page.getByLabel(t("field.name")).fill(name);
  await page.getByLabel(t("field.email")).fill(email);
  await page.getByLabel(t("field.password")).fill(password);
  await page.getByRole("button", { name: t("sign-up.submit") }).click();
}

test("auth_06 signing up is off by default: the page says so and the sign-in page has no link", async ({
  page,
}) => {
  await page.goto("/sign-in");
  await expect(page.getByRole("heading", { level: 1, name: t("sign-in.title") })).toBeVisible();
  await expect(page.getByRole("link", { name: t("sign-in.make-account") })).toHaveCount(0);
  await page.goto("/sign-up");
  await expect(page.getByText(t("sign-up.closed"))).toBeVisible();
  await expect(page.getByLabel(t("field.email"))).toHaveCount(0);
});

test("m1_f46 an administrator switches it on and is warned before it is open to everybody", async ({
  page,
}) => {
  const policy = await openPolicy(page);
  await setSwitch(policy, true);
  // Switched on and no domain listed: everybody could sign up, and the page says so.
  await expect(policy.getByText(t("sign-up-policy.warning"))).toBeVisible();
  await policy.getByRole("button", { name: t("common.save") }).click();
  const question = page.getByRole("alertdialog");
  await expect(question).toContainText(t("sign-up-policy.confirm-text"));
  await question.getByRole("button", { name: t("sign-up-policy.confirm-no") }).click();
  await expect(question).toHaveCount(0);

  // With a domain the warning is gone and saving needs no question.
  await policy.getByLabel(t("sign-up-policy.domains")).fill("example.test");
  await expect(policy.getByText(t("sign-up-policy.warning"))).toHaveCount(0);
  await policy.getByRole("button", { name: t("common.save") }).click();
  await expect(policy.getByText(t("sign-up-policy.saved"))).toBeVisible();
  await signOut(page);
});

test("auth_06 people of the listed domain make their own account, and others are refused", async ({
  page,
}) => {
  await page.goto("/sign-in");
  await page.getByRole("link", { name: t("sign-in.make-account") }).click();
  await expect(page.getByRole("heading", { level: 1, name: t("sign-up.title") })).toBeVisible();

  await sendSignUp(page, "Mallory", "mallory@elsewhere.test", "Maple-Harvest-Quartz-2291");
  await expect(page.getByRole("alert")).toHaveText(t("error.auth.sign-up-refused"));

  await sendSignUp(page, bob.name, bob.email, bob.password);
  await expectHome(page, bob);
});

test("auth_06 an administrator can switch it off again", async ({ page }) => {
  const policy = await openPolicy(page);
  await setSwitch(policy, false);
  await policy.getByRole("button", { name: t("common.save") }).click();
  await expect(policy.getByText(t("sign-up-policy.saved"))).toBeVisible();
  await signOut(page);
  await expect(page.getByRole("link", { name: t("sign-in.make-account") })).toHaveCount(0);
});
