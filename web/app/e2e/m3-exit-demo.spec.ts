import { theBugIsRelatedToTwoIssues } from "./support/demo-m3-after.ts";
import { twoPeopleEditTheDescription } from "./support/demo-m3-edit.ts";
import { theGuestSeesSomeone } from "./support/demo-m3-guest.ts";
import { aGuestMakesABug, aTeamMemberMakesABug } from "./support/demo-m3-create.ts";
import { prepareTheDemo } from "./support/demo-m3-prepare.ts";
import { test } from "./support/fixtures.ts";

// The exit demo of M3 (docs/04-milestones.md) on the sample organization, as the maintainer can repeat
// it by hand with `cargo xtask dev --sample`: a Guest makes a bug with the short form; a Team
// member makes one with the full form (with Cost center, which the Guest does not see, and a
// person assigned, whom the Guest sees as Someone); the bug is related to two issues; and two
// people edit its description at once (the lock, Edit anyway, Compare and save). Every step is
// good in the four themes on a phone and on a computer, and with `BB_SCREENSHOT_DIR` set the
// same run takes the pictures of the review page
// (`<folder>/M3/demo/<step>-<phone|computer>-<theme>.png`). The steps are in `support/demo-m3-*.ts`.

test.use({ sample: true, preparation: { run: prepareTheDemo } });
test.describe.configure({ mode: "serial" });

test("1. A Guest makes a bug with the short form", async ({ page }) => {
  test.setTimeout(180_000);
  await aGuestMakesABug(page);
});

test("2. A Team member makes a bug with the full form, with Cost center and a person assigned", async ({
  page,
}) => {
  test.setTimeout(180_000);
  await aTeamMemberMakesABug(page);
});

test("3. The bug is related to two issues", async ({ page }) => {
  test.setTimeout(180_000);
  await theBugIsRelatedToTwoIssues(page);
});

test("4. The Guest sees Someone for the person assigned, and no Cost center", async ({
  page,
  server,
}) => {
  test.setTimeout(180_000);
  await theGuestSeesSomeone(page, server);
});

test("5. Two people edit the description at once: the lock, Edit anyway, Compare and save", async ({
  browser,
  server,
  viewport,
  hasTouch,
}) => {
  test.setTimeout(240_000);
  await twoPeopleEditTheDescription(browser, server, { viewport, hasTouch });
});
