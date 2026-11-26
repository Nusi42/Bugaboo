import { t } from "@bugaboo/messages";

import { paths } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { expectNoOverlap, inEveryLook } from "./support/checks.ts";
import { boxOnScreen, readyMadeTitle } from "./support/dashboards.ts";
import { expect, test } from "./support/fixtures.ts";
import { apiOf, signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";
import { longScene } from "./support/time-scene.ts";

// What a box holds stays inside its own columns (UX §4.10): the box *My timer* of *Home*, half the width of the grid,
// with the long warning of a timer that has run longer than a work day and its buttons, is drawn over nothing beside it,
// in the four themes, on a computer and a phone.

test.use({ preparation: { run: prepareWeb } });
test.setTimeout(180_000);

test("cfg_03_the_warning_of_a_long_timer_stays_inside_its_box_on_home", async ({
  page,
  server,
}) => {
  const { issue, reset, dispose } = await longScene(server);
  await signIn(page, ada);
  await page.goto(paths.home);
  const timer = boxOnScreen(page, await readyMadeTitle(apiOf(page, server), "my-timer"));
  await expect(timer).toContainText(t("time.timer.runs-long-on", { issue: issue.key }));
  await inEveryLook(page, async () => {
    await expectNoOverlap(page);
  });
  await reset();
  await dispose();
});
