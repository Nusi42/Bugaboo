import { testEmailEverySeconds } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { type Page } from "@playwright/test";

import { giveRole, roleName } from "./support/access.ts";
import { ada, cleo, createAdmin, createPerson, ownId, type Person } from "./support/accounts.ts";
import { expectGoodInEveryLook } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { openFromMenu, signIn, signOut } from "./support/flows.ts";
import { formFor, fillMailServer, mailRejectedLine, sendTestEmail } from "./support/mail-form.ts";
import { type Login, type Mailbox, startMailbox } from "./support/mailbox.ts";

// The mail server of the organization (NTF-02, PRIV-08, M4 WP-08): an administrator enters it,
// tries it with *Send me a test e-mail*, and switches e-mail on. The mail server of the test asks
// for a user name and a password, so a wrong password can be refused.

const login: Login = { user: "mailer", password: "the-right-one" };
const wrong = "a-wrong-one";

let mailbox: Mailbox;

test.use({
  preparation: {
    run: async (server) => {
      const admin = await createAdmin(server);
      mailbox = await startMailbox(login);
      // A second administrator: a person sends a test e-mail every ten seconds at most, so the
      // second try is made by somebody else.
      const second = await createPerson(server, admin, cleo);
      await giveRole(admin, await ownId(second), roleName.administrator, null);
      await second.dispose();
      await admin.dispose();
    },
  },
});
test.describe.configure({ mode: "serial" });

/** Opens the page of the mail server the way an administrator does: from the menu. */
async function openMailServer(page: Page, person: Person): Promise<void> {
  await signIn(page, person);
  await openFromMenu(page, t("nav.mail-server"));
  await expect(
    page.getByRole("heading", { level: 1, name: t("mail-server.title"), exact: true }),
  ).toBeVisible();
}

test("ntf_02 e-mail is off at the start, and a test without a mail server says so", async ({
  page,
}) => {
  await openMailServer(page, ada);
  await expect(
    page.getByRole("switch", { name: t("setting.mail.enabled.name") }),
  ).not.toBeChecked();
  await expect(page.getByText(t("mail-server.password-none"))).toBeVisible();
  await sendTestEmail(page);
  await expect(page.getByRole("alert")).toContainText(t("error.mail.off"));
});

test("ntf_02 a wrong password is refused in words, and the button then waits", async ({
  page,
  tolerate,
}) => {
  const refusal = tolerate(page.context(), mailRejectedLine, { network: "stays-up" });
  await openMailServer(page, ada);
  await fillMailServer(page, formFor(mailbox, { user: login.user, password: wrong }));
  await expect(page.getByText(t("mail-server.password-set"))).toBeVisible();
  await refusal.begin();
  await sendTestEmail(page);
  await expect(page.getByRole("alert")).toContainText(t("error.mail.rejected"));
  await refusal.end();
  // The server lets one person send a test e-mail every ten seconds; the button waits that long.
  await expect(
    page.getByRole("button", { name: t("mail-server.test-send"), exact: true }),
  ).toBeDisabled();
  expect(mailbox.mails()).toEqual([]);

  // A page that was opened again does not know about the wait, and the server says it: too soon.
  await page.reload();
  await sendTestEmail(page);
  await expect(
    page.getByRole("region", { name: t("mail-server.test-title") }).getByRole("status"),
  ).toContainText(t("mail-server.test-wait", { seconds: testEmailEverySeconds }));
});

test("ntf_02 a user name or a password without encryption is refused for another machine", async ({
  page,
}) => {
  await openMailServer(page, ada);
  await fillMailServer(page, {
    ...formFor(mailbox, login),
    host: "mail.example.invalid",
  });
  await sendTestEmail(page);
  await expect(page.getByRole("alert")).toContainText(t("error.mail.insecure"));
  // The page says the rule before the test does.
  await expect(page.getByText(t("mail-server.no-encryption-note"))).toBeVisible();
});

test("ntf_02 with the right password the test e-mail reaches the mail server, and e-mail can be switched on", async ({
  page,
}) => {
  await openMailServer(page, cleo);
  await fillMailServer(page, formFor(mailbox, login));
  await sendTestEmail(page);
  await expect(page.getByText(t("mail-server.test-done", { email: cleo.email }))).toBeVisible();
  const mail = await mailbox.waitFor(cleo.email);
  expect(mail.text).toContain("test e-mail");

  const sending = page.getByRole("switch", { name: t("setting.mail.enabled.name") });
  await page.getByText(t("setting.mail.enabled.name"), { exact: true }).click();
  await expect(page.getByText(t("save.saved"), { exact: true })).toBeVisible();
  // The page is reloaded only when the server has the second change.
  const saved = page.waitForResponse(/\/api\/v1\/settings\/mail\.short$/u);
  await page.getByText(t("setting.mail.short.name"), { exact: true }).click();
  await saved;
  await page.reload();
  await expect(sending).toBeChecked();
  await expect(page.getByRole("switch", { name: t("setting.mail.short.name") })).toBeChecked();
  await expect(page.getByText(t("mail-server.password-set"))).toBeVisible();
});

test("ntf_02 the page is good in every look", async ({ page }) => {
  await openMailServer(page, ada);
  await expectGoodInEveryLook(page);
  await signOut(page);
});
