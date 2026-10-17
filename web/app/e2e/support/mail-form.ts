import { t } from "@bugaboo/messages";
import { expect, type Page } from "@playwright/test";

import { type Login, type Mailbox } from "./mailbox.ts";

/** What the person types into the form of the mail server. */
export interface MailForm {
  readonly host: string;
  readonly port: number;
  readonly from: string;
  readonly security: "off" | "starttls" | "tls";
  readonly login?: Login;
}

/** The page of the mail server, the way it is filled: the fields by their names, then *Save*. */
export async function fillMailServer(page: Page, form: MailForm): Promise<void> {
  await page.getByLabel(t("setting.mail.host.name"), { exact: true }).fill(form.host);
  await page.getByLabel(t("setting.mail.port.name"), { exact: true }).fill(String(form.port));
  await page.getByLabel(t("setting.mail.from.name"), { exact: true }).fill(form.from);
  await page
    .getByLabel(t("setting.mail.username.name"), { exact: true })
    .fill(form.login?.user ?? "");
  await page
    .getByLabel(t("mail-server.password-new"), { exact: true })
    .fill(form.login?.password ?? "");
  await page.getByRole("radio", { name: t(`setting.mail.security.${form.security}`) }).check();
  await page.getByRole("button", { name: t("mail-server.save"), exact: true }).click();
  await expect(page.getByText(t("save.saved"), { exact: true })).toBeVisible();
}

/** The form of a mail server on this computer, the one a test starts (`startMailbox`). */
export function formFor(mailbox: Mailbox, login?: Login): MailForm {
  return {
    host: "127.0.0.1",
    port: mailbox.port,
    from: "bugaboo@example.test",
    // Nothing to protect on this computer, so the server allows a login without encryption.
    security: "off",
    ...(login === undefined ? {} : { login }),
  };
}

/** Presses *Send me a test e-mail*. */
export async function sendTestEmail(page: Page): Promise<void> {
  await page.getByRole("button", { name: t("mail-server.test-send"), exact: true }).click();
}
