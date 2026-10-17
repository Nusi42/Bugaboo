import { t } from "@bugaboo/messages";
import { type Locator, type Page } from "@playwright/test";

import { type ApiSession } from "./support/api.ts";
import { ada, bob, createAdmin } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { expectHome, openFromMenu, signIn, signOut } from "./support/flows.ts";
import { invitationPath, type Mailbox, startMailbox, turnMailOn } from "./support/mailbox.ts";
import { paths } from "../src/shared/paths.ts";

/** The administrator, signed in through the API, and the mail server the Bugaboo server sends to. */
let admin: ApiSession;
let mailbox: Mailbox;

test.use({
  preparation: {
    run: async (server) => {
      admin = await createAdmin(server);
      mailbox = await startMailbox();
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
  await openFromMenu(page, t("nav.people"));
  return page.getByRole("region", { name: t("sign-up-policy.title") });
}

/** Fills in the sign-up page and sends it. */
async function sendSignUp(page: Page, email: string): Promise<void> {
  await page.getByLabel(t("field.email")).fill(email);
  await page.getByRole("button", { name: t("sign-up.submit") }).click();
}

test("auth_06 signing up is off by default: the page says so and the sign-in page has no link", async ({
  page,
}) => {
  await page.goto(paths.signIn);
  await expect(page.getByRole("heading", { level: 1, name: t("sign-in.title") })).toBeVisible();
  await expect(page.getByRole("link", { name: t("sign-in.create-account") })).toHaveCount(0);
  await page.goto(paths.signUp);
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
  await expect(policy.getByText(t("save.saved"))).toBeVisible();
  await signOut(page);
});

test("auth_06 signing up needs e-mail: with it off the sign-in page has no link and the page says so", async ({
  page,
}) => {
  // Signing up is on, for the domain, and no mail server is entered.
  await page.goto(paths.signIn);
  await expect(page.getByRole("heading", { level: 1, name: t("sign-in.title") })).toBeVisible();
  await expect(page.getByRole("link", { name: t("sign-in.create-account") })).toHaveCount(0);
  await page.goto(paths.signUp);
  await expect(page.getByText(t("sign-up.closed"))).toBeVisible();
  await expect(page.getByLabel(t("field.email"))).toHaveCount(0);
});

test("auth_06 people of the listed domain get a link by e-mail and make their own account with it", async ({
  page,
}) => {
  await turnMailOn(admin, mailbox);
  await page.goto(paths.signIn);
  await page.getByRole("link", { name: t("sign-in.create-account") }).click();
  await expect(page.getByRole("heading", { level: 1, name: t("sign-up.title") })).toBeVisible();

  // Another domain is refused, and nothing is sent.
  await sendSignUp(page, "mallory@elsewhere.test");
  await expect(page.getByRole("alert")).toHaveText(t("error.auth.sign-up-refused"));

  // The listed domain is told to look at its mail; nothing is made yet.
  await sendSignUp(page, bob.email);
  await expect(page.getByText(t("sign-up.sent"))).toBeVisible();
  const mail = await mailbox.waitFor(bob.email);
  expect(mailbox.mails().some((sent) => sent.to === "mallory@elsewhere.test")).toBe(false);

  // The link of the e-mail is the invitation page: the person chooses a name and a password there.
  await page.goto(invitationPath(mail));
  await expect(page.getByRole("heading", { level: 1, name: t("invitation.title") })).toBeVisible();
  await page.getByLabel(t("field.name")).fill(bob.name);
  await page.getByLabel(t("field.password")).fill(bob.password);
  await page.getByRole("button", { name: t("invitation.submit") }).click();
  await expectHome(page, bob);
  await signOut(page);
});

test("auth_06 an address that has an account is told the same, and gets no link", async ({
  page,
}) => {
  await page.goto(paths.signUp);
  await sendSignUp(page, ada.email);
  await expect(page.getByText(t("sign-up.sent"))).toBeVisible();
  const mail = await mailbox.waitFor(ada.email);
  expect(mail.text).not.toContain("/invitation#");
});

test("auth_06 an administrator can switch it off again", async ({ page }) => {
  const policy = await openPolicy(page);
  await setSwitch(policy, false);
  await policy.getByRole("button", { name: t("common.save") }).click();
  await expect(policy.getByText(t("save.saved"))).toBeVisible();
  await signOut(page);
  await expect(page.getByRole("link", { name: t("sign-in.create-account") })).toHaveCount(0);
});
