import { test } from "./support/fixtures.ts";
import { prepareWeb } from "./support/issues.ts";
import { shootMailServer, shotLogin } from "./support/mail-server-shots.ts";
import { startMailbox, type Mailbox } from "./support/mailbox.ts";
import { shootNotifications } from "./support/notification-shots.ts";
import { screenshotFolder } from "./support/screenshots.ts";
import { prepareShots, shootSignUp } from "./support/sign-up-shots.ts";

// The screens of notifications for the maintainer's review (M4 WP-08), in the four themes, as a computer
// and as a phone show them. They are only taken when `BB_SCREENSHOT_DIR` names the folder:
//   BB_SCREENSHOT_DIR=... npx playwright test screenshots-notifications --project=desktop --project=phone
test.skip(screenshotFolder === undefined, "Set BB_SCREENSHOT_DIR to take the screenshots.");

let mailbox: Mailbox;

test.use({
  preparation: {
    run: async (server) => {
      await prepareWeb(server);
      mailbox = await startMailbox(shotLogin);
      await prepareShots(server);
    },
  },
});
test.describe.configure({ mode: "serial" });

/** All the themes of one screen take longer than an ordinary test. */
const ALL_THE_THEMES_MS = 300_000;

test("the notifications, the start page, Customize and the handle", async ({ page, server }) => {
  test.setTimeout(ALL_THE_THEMES_MS);
  await shootNotifications(page, server);
});

test("the mail server: at the start, refused, and tried", async ({ page }) => {
  test.setTimeout(ALL_THE_THEMES_MS);
  await shootMailServer(page, mailbox);
});

test("signing up: the form and Check your e-mail", async ({ page }) => {
  test.setTimeout(ALL_THE_THEMES_MS);
  await shootSignUp(page);
});
