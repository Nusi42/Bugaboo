import { t } from "@bugaboo/messages";
import { expect, type Page } from "@playwright/test";

import { ada, cleo } from "./accounts.ts";
import { openFromMenu, signIn, signOut } from "./flows.ts";
import { fillMailServer, formFor, sendTestEmail } from "./mail-form.ts";
import { type Login, type Mailbox } from "./mailbox.ts";
import { inEveryTheme } from "./shots-themes.ts";

/** The mail server of the pictures asks for this login. */
export const shotLogin: Login = { user: "mailer", password: "the-right-one" };

/** The mail server: at the start, refused for a wrong password, tried and switched on. */
export async function shootMailServer(page: Page, mailbox: Mailbox): Promise<void> {
  await signIn(page, ada);
  await openFromMenu(page, t("nav.mail-server"));
  await expect(page.getByRole("heading", { level: 1, name: t("mail-server.title") })).toBeVisible();
  await inEveryTheme(page, "10-mail-server-at-the-start");
  await fillMailServer(page, formFor(mailbox, { ...shotLogin, password: "a-wrong-one" }));
  await sendTestEmail(page);
  await expect(page.getByRole("alert")).toContainText(t("error.mail.rejected"));
  await inEveryTheme(page, "11-mail-server-refused");
  await signOut(page);
  await signIn(page, cleo);
  await openFromMenu(page, t("nav.mail-server"));
  await fillMailServer(page, formFor(mailbox, shotLogin));
  await sendTestEmail(page);
  await expect(page.getByText(t("mail-server.test-done", { email: cleo.email }))).toBeVisible();
  await page.getByText(t("setting.mail.enabled.name"), { exact: true }).click();
  await expect(page.getByText(t("save.saved"), { exact: true })).toBeVisible();
  await inEveryTheme(page, "12-mail-server-tried-and-on");
}
