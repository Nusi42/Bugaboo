import { setTimeout as sleep } from "node:timers/promises";

import { dashboardAddress } from "../src/shared/places.ts";
import { ada, signInThroughApi } from "./support/accounts.ts";
import { readyMadeBox } from "./support/dashboard-issues.ts";
import { createDashboard, resultsBox } from "./support/dashboards.ts";
import { expect, test } from "./support/fixtures.ts";
import { firstReadOf } from "./support/first-read.ts";
import { signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";

// The first read of a dashboard (UX §6.4): a box whose screen is loaded when it is shown (the small timeline) is waited
// for with the reads of the boxes, so from the first frame of the page no box grows and pushes the boxes under it.

test.use({ preparation: { run: prepareWeb } });

/** How long the code of the small timeline takes to arrive: slower than the reads, so the page could be seen first. */
const SLOW_CODE_MS = 1500;

test("cfg_03_a_small_timeline_box_moves_nothing_from_the_first_frame", async ({ page, server }) => {
  const admin = await signInThroughApi(server, ada);
  const timeline = await readyMadeBox(admin, "small-timeline-of-a-search");
  const made = await createDashboard(admin, "First frame", [
    Object.assign(timeline, { column: 0, row: 0, width: 12, height: 2 }),
    resultsBox("Under the timeline", { column: 0, row: 2, width: 12, height: 2 }),
  ]);
  await admin.dispose();
  await signIn(page, ada);
  await page.route(/\/assets\/timeline-[^/]*\.js$/u, async (route) => {
    await sleep(SLOW_CODE_MS);
    await route.continue();
  });
  expect(await firstReadOf(page, dashboardAddress(made.id))).toEqual([]);
});
