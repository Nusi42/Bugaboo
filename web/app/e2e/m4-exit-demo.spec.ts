import { prepareTheDemo } from "./support/demo-m4-prepare.ts";
import { theGuestHearsByMail, theGuestHearsInTheApp } from "./support/demo-m4-guest.ts";
import { aScreenshotIsPasted } from "./support/demo-m4-screenshot.ts";
import { startingTheWorkAsksForWho, theBugIsDone, theBugIsOpen } from "./support/demo-m4-work.ts";
import { test } from "./support/fixtures.ts";

// The exit demo of M4 (docs/04-milestones.md) on the sample organization, as the maintainer can repeat it
// by hand with `cargo xtask dev --sample`: a Guest has reported a bug in Operations and follows it;
// a Team member moves it from Open to Done through its workflow (starting the work asks for who does
// it), pastes a screenshot onto it, and the Guest hears of it in the app and by e-mail, without the
// customer and without the person who works on it, which a Guest may not see. Every step is good in
// the four themes on a phone and on a computer, and with `BB_SCREENSHOT_DIR` set the same run takes
// the pictures of the review page (`<folder>/M4/demo/<step>-<phone|computer>-<theme>.png`). The
// steps are in `support/demo-m4-*.ts`.

test.use({
  sample: true,
  // Changes to one issue wait two seconds for each other, not five minutes, so the e-mail comes
  // while the test waits for it (mail_batch_seconds).
  serverSettings: { BUGABOO_MAIL_BATCH_SECONDS: "2" },
  preparation: { run: prepareTheDemo },
});
test.describe.configure({ mode: "serial" });

test("1. The bug of a Guest is open in Operations", async ({ page }) => {
  test.setTimeout(180_000);
  await theBugIsOpen(page);
});

test("2. Starting the work asks for who does it", async ({ page }) => {
  test.setTimeout(180_000);
  await startingTheWorkAsksForWho(page);
});

test("3. A screenshot is pasted onto the bug", async ({ page, server }) => {
  test.setTimeout(180_000);
  await aScreenshotIsPasted(page, server);
});

test("4. The bug is marked as done", async ({ page }) => {
  test.setTimeout(180_000);
  await theBugIsDone(page);
});

test("5. The Guest finds the news in the app, without the customer and without the person", async ({
  page,
}) => {
  test.setTimeout(180_000);
  await theGuestHearsInTheApp(page);
});

test("6. The e-mail has come to the Guest too, without the customer and without the person", async ({
  page,
}) => {
  test.setTimeout(180_000);
  await theGuestHearsByMail(page);
});
