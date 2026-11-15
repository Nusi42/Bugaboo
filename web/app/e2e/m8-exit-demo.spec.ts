import { STEP_TIMEOUT_MS } from "./support/demo-m8-common.ts";
import {
  aShareOfHalfDoublesLeasBars,
  leasAwayWeekGrowsHerBarsAndWarns,
} from "./support/demo-m8-lea.ts";
import {
  anOwnThemeThatIsHardToReadIsFixed,
  givenToSomeoneElseTheFactorGoesBack,
  leaSaysAnIssueTakesHerTenDays,
} from "./support/demo-m8-own.ts";
import { theGoalIsPlannedByDragging } from "./support/demo-m8-plan.ts";
import { test } from "./support/fixtures.ts";

// The exit demo of M8 (docs/04-milestones.md, with M8-Q3) on the sample organization, as the
// maintainer can repeat it by hand with `cargo xtask dev --sample`: Ada plans *Mobile app 2.0* by
// dragging a bar; a week away of Lea grows her bars around it and warns that Login screen ends
// after the due date of the goal; a share of 50 % doubles her bars; as Lea an issue takes ten days
// and shows as her own estimate; given to someone else, its time factor goes back to ×1; and a
// theme for everyone that is hard to read is refused until it is fixed. Every step is good in the
// four themes on a phone and on a computer, and with `BB_SCREENSHOT_DIR` set the same run takes the
// pictures (`<folder>/M8/exit-demo/<step>-<phone|computer>-<theme>.png`).

test.use({ sample: true });
test.describe.configure({ mode: "serial" });

test("1. Mobile app 2.0 is planned by dragging", async ({ page, server }) => {
  test.setTimeout(STEP_TIMEOUT_MS);
  await theGoalIsPlannedByDragging(page, server);
});

test("2. Lea's away week grows her bars and warns about the goal's due date", async ({
  page,
  server,
}) => {
  test.setTimeout(STEP_TIMEOUT_MS);
  await leasAwayWeekGrowsHerBarsAndWarns(page, server);
});

test("3. A share of 50 % doubles Lea's bars", async ({ page, server }) => {
  test.setTimeout(STEP_TIMEOUT_MS);
  await aShareOfHalfDoublesLeasBars(page, server);
});

test("4. Lea says an issue takes her 10 days and it shows as her own estimate", async ({
  page,
  server,
}) => {
  test.setTimeout(STEP_TIMEOUT_MS);
  await leaSaysAnIssueTakesHerTenDays(page, server);
});

test("5. Given to someone else, the time factor goes back to ×1", async ({ page, server }) => {
  test.setTimeout(STEP_TIMEOUT_MS);
  await givenToSomeoneElseTheFactorGoesBack(page, server);
});

test("6. An own theme that is hard to read is fixed and saved for everyone", async ({
  page,
  server,
}) => {
  test.setTimeout(STEP_TIMEOUT_MS);
  await anOwnThemeThatIsHardToReadIsFixed(page, server);
});
