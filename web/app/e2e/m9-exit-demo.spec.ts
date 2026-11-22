import { aboutTheRelease } from "./support/demo-m9-about.ts";
import { aBackupMadeWhilePeopleWorkIsRestored } from "./support/demo-m9-backup.ts";
import { adaThroughApi, STEP_TIMEOUT_MS } from "./support/demo-m9-common.ts";
import { leaAndAnAdministratorDownloadData } from "./support/demo-m9-data.ts";
import { benIsRemoved } from "./support/demo-m9-remove.ts";
import { test } from "./support/fixtures.ts";
import { type Mailbox, startMailbox, turnMailOn } from "./support/mailbox.ts";

// The exit demo of M9 (docs/04-milestones.md, broken into steps in docs/milestones/M9/README.md) on the
// sample organization, as it can be repeated by hand with `cargo xtask dev --sample`: steps 1 to 3 are
// the release and its try-out runs (xtask), step 8 is the upgrade test (`cargo xtask upgrade-test`,
// and `exit_demo_m9.rs` of `bb-server`); steps 4 to 7 are here. A backup made while people work is
// restored, Lea downloads her data and an administrator downloads Ben's, Ben is removed, and About
// shows the release. Every step is good in the four themes on a phone and on a computer, and with
// `BB_SCREENSHOT_DIR` set the same run takes the pictures (`<folder>/M9/exit-demo/<step>-<phone|computer>-<theme>.png`).

let mailbox: Mailbox;

test.use({
  sample: true,
  preparation: {
    run: async (server) => {
      // Ben is told by e-mail when an administrator downloads his data (step 5).
      mailbox = await startMailbox();
      const admin = await adaThroughApi(server);
      await turnMailOn(admin, mailbox);
      await admin.dispose();
    },
  },
});
test.describe.configure({ mode: "serial" });

test("4. A backup made while people work is restored", async ({ page, server }) => {
  test.setTimeout(STEP_TIMEOUT_MS);
  await aBackupMadeWhilePeopleWorkIsRestored(page, server);
});

test("5. Lea downloads her data, and an administrator downloads Ben's", async ({ page }) => {
  test.setTimeout(STEP_TIMEOUT_MS);
  await leaAndAnAdministratorDownloadData(page, mailbox);
});

test("6. Ben is removed", async ({ page, server }) => {
  test.setTimeout(STEP_TIMEOUT_MS);
  await benIsRemoved(page, server);
});

test("7. The About page shows the release", async ({ page, server }) => {
  test.setTimeout(STEP_TIMEOUT_MS);
  await aboutTheRelease(page, server);
});
