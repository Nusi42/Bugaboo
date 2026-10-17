import { t } from "@bugaboo/messages";
import { type Locator, type Page } from "@playwright/test";

import { ana, signedInAdmin } from "./support/access.ts";
import { expectGoodInEveryLook } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { apiOf, signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";
import { type Mailbox, startMailbox, turnMailOn } from "./support/mailbox.ts";
import {
  asBen,
  comment,
  expectUnread,
  follow,
  handleOf,
  openNotifications,
} from "./support/notifications.ts";
import { paths } from "../src/shared/paths.ts";

// Customize and the e-mail of notifications (NTF-01, NTF-02, M4 WP-08): one row for each kind of
// event with its two switches, and the e-mail that comes, or does not come, because of them.

let mailbox: Mailbox;

test.use({
  // Changes to one issue wait two seconds for each other, not five minutes, so the e-mail comes
  // while the test waits for it (mail_batch_seconds).
  serverSettings: { BUGABOO_MAIL_BATCH_SECONDS: "2" },
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

/** The row of the changes, comments and files of the issues Ana follows. */
function followedRow(page: Page): Locator {
  return page.getByRole("group", { name: t("notification.kind.followed-activity") });
}

/** Flips one switch of a row with a click on its words, as a person does. */
async function flip(row: Locator, way: string): Promise<void> {
  await row.getByText(way, { exact: true }).click();
}

/** The e-mails Ana got that say this text. */
function mailsTo(text: string): number {
  return mailbox.mails().filter((mail) => mail.to === ana.email && mail.text.includes(text)).length;
}

test("ntf_02 Customize has a row for each kind of event, with the standard choices of the start", async ({
  page,
}) => {
  await signIn(page, ana);
  await openNotifications(page);
  await page.getByRole("link", { name: t("notifications.customize"), exact: true }).click();
  await expect(
    page.getByRole("heading", { level: 1, name: t("notifications.customize-title") }),
  ).toBeVisible();
  const mentioned = page.getByRole("group", { name: t("notification.kind.mentioned") });
  await expect(mentioned.getByRole("switch", { name: t("notifications.in-app") })).toBeChecked();
  await expect(mentioned.getByRole("switch", { name: t("notifications.by-mail") })).toBeChecked();
  const followed = followedRow(page);
  await expect(followed.getByRole("switch", { name: t("notifications.in-app") })).toBeChecked();
  await expect(
    followed.getByRole("switch", { name: t("notifications.by-mail") }),
  ).not.toBeChecked();
  await expectGoodInEveryLook(page);
});

test("ntf_02 Customize turns e-mail on and off for what Ana follows, and the next comment sends nothing", async ({
  page,
  browser,
  server,
}) => {
  await signIn(page, ana);
  const anaApi = apiOf(page, server);
  await follow(anaApi, "WEB-1");
  const benApi = apiOf(await asBen(browser, server), server);

  await page.goto(paths.notificationSettings);
  await flip(followedRow(page), t("notifications.by-mail"));
  await expect(page.getByText(t("save.saved"))).toBeVisible();
  await page.reload();
  await expect(
    followedRow(page).getByRole("switch", { name: t("notifications.by-mail") }),
  ).toBeChecked();

  // With e-mail on, the comment on an issue Ana follows comes by e-mail.
  await comment(benApi, "WEB-1", "A second note, which comes by e-mail.");
  await expect.poll(() => mailsTo("A second note")).toBe(1);

  // Customize turns it off again.
  await flip(followedRow(page), t("notifications.by-mail"));
  await expect(
    followedRow(page).getByRole("switch", { name: t("notifications.by-mail") }),
  ).not.toBeChecked();
  await expect(page.getByText(t("save.saved"))).toBeVisible();

  // The next comment sends nothing. The e-mail about a mention goes at the same moment and takes
  // along every note of the issue that waits, so when it has come without the note, the note was
  // never going to be sent.
  await comment(benApi, "WEB-1", "A third note, which is not for the mail.");
  await comment(benApi, "WEB-1", `And a mention for the mail, @${await handleOf(anaApi)}`);
  await expect.poll(() => mailsTo("And a mention for the mail")).toBe(1);
  expect(mailsTo("A third note")).toBe(0);
  // The note is in the app all the same.
  await openNotifications(page);
  await expectUnread(page, 3);
  await expect(page.getByText("A third note, which is not for the mail.")).toBeVisible();
});
