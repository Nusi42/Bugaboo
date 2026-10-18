import { t } from "@bugaboo/messages";
import { expect, type Page } from "@playwright/test";

import { theTwo } from "./demo-m3-state.ts";
import { good, openTheBug } from "./demo-m4-page.ts";
import { BUG, CUSTOMER, guest, made } from "./demo-m4-state.ts";
import { signIn } from "./flows.ts";
import { openNotifications } from "./notifications.ts";

// The steps of the exit demo of M4 that are the Guest's (`m4-exit-demo.spec.ts`): the Guest follows
// the bug, and hears of it in the app and by e-mail without what a Guest may not see.

/** Step 5: the Guest finds the news in the app, without the customer and without the person. */
export async function theGuestHearsInTheApp(page: Page): Promise<void> {
  await signIn(page, guest);
  await openNotifications(page);
  await expect(page.getByText(BUG).first()).toBeVisible();
  await expect(page.getByText(t("status.done")).first()).toBeVisible();
  await expect(page.getByText(CUSTOMER)).toHaveCount(0);
  await expect(page.getByText(theTwo().second.name)).toHaveCount(0);
  await good(page, "06-the-guest-hears-in-the-app");
}

/** Step 6: the e-mail has come to the Guest too, without the customer and without the person. */
export async function theGuestHearsByMail(page: Page): Promise<void> {
  const mailbox = made.mailbox;
  if (mailbox === undefined) {
    throw new Error("The mail server of the demo was not started.");
  }
  const toTheGuest = () => mailbox.mails().filter((mail) => mail.to === guest.email);
  await expect.poll(() => toTheGuest().some((mail) => mail.text.includes(BUG))).toBe(true);
  for (const mail of toTheGuest()) {
    expect(mail.text).not.toContain(CUSTOMER);
    expect(mail.text).not.toContain(theTwo().second.name);
  }
  // The links of the e-mail open the bug of the Guest.
  await signIn(page, guest);
  await openTheBug(page);
  await good(page, "07-the-guest-opens-the-bug");
}
