import { t } from "@bugaboo/messages";

import { paths } from "../src/shared/paths.ts";
import { bob } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { addAwayTime, comingAway, dayFromToday, prepareWorkTimes } from "./support/work-times.ts";

// Work times on a phone (UX-02, UX §7.4): *Change* and *Delete* of an away time are a finger high.

test.use({ preparation: { run: prepareWorkTimes } });

/** A finger's target on touch (UX §7.4, `size-control` on touch). */
const FINGER_PX = 44;

test("ux_02 Change and Delete of an away time are at least 44 px high on a phone", async ({
  page,
}) => {
  await signIn(page, bob);
  await page.goto(paths.workTimes);
  await addAwayTime(page, { from: dayFromToday(5), to: dayFromToday(6) });
  const row = comingAway(page).getByRole("listitem");
  const change = await row.getByRole("button", { name: t("work-times.away-change") }).boundingBox();
  const remove = await row.getByRole("button", { name: t("work-times.away-delete") }).boundingBox();
  expect(change?.height).toBeGreaterThanOrEqual(FINGER_PX);
  expect(remove?.height).toBeGreaterThanOrEqual(FINGER_PX);
});
