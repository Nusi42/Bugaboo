import type { Page } from "@playwright/test";

import { dataOf } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";

import { signInThroughApi } from "./support/accounts.ts";
import {
  adaOfTheSample,
  openPlanAt,
  plan,
  planSearch,
  withoutATimer,
} from "./support/demo-m8-common.ts";
import { expect, test } from "./support/fixtures.ts";
import { mediaFolder, samplePage, shootPicture } from "./support/readme-media.ts";
import { showReadmeTheme } from "./support/readme-organization.ts";
import { savedSearchAddress } from "../src/shared/paths.ts";

// The picture of the timeline on the front page (README.md): the plan of *Mobile app 2.0* of the sample organization,
// grouped by goal and planned by effort, in the theme of the other pictures. Quillmoor Cycle Works has no plan, so this
// picture is made on the sample. `npm run readme-media` runs this file with `readme-media.spec.ts`.

/**
 * Scrolls the chart so that today stands near the start of its days: the plan lies after today, and so its bars and
 * the due date of the goal show. A unit that would start so near the end of the box that its name is cut there is left
 * just out of sight.
 */
async function todayAtTheStart(page: Page): Promise<void> {
  await page.locator("[data-part='timeline-scroll']").evaluate((box) => {
    const place = (part: string) =>
      box.querySelector(`[data-part~='${part}']`)?.getBoundingClientRect();
    const today = place("timeline-today")?.left ?? 0;
    const names = place("timeline-head.corner");
    box.scrollBy({ left: today - (names?.right ?? 0) - (names?.width ?? 0) / 2 });
    const end = box.getBoundingClientRect().left + box.clientLeft + box.clientWidth;
    for (const name of box.querySelectorAll("[data-part='day-scale.name']")) {
      const shown = name.getBoundingClientRect();
      if (shown.left < end && shown.right > end) {
        box.scrollBy({ left: shown.left - end - 1 });
      }
    }
  });
}

test.skip(mediaFolder === undefined, "Set BB_README_MEDIA_DIR to make the pictures.");
test.use({ sample: true });

test("a picture of the timeline: a goal and its issues as bars", async ({ browser, server }) => {
  const ada = adaOfTheSample();
  await withoutATimer(server, ada);
  const session = await signInThroughApi(server, ada);
  await showReadmeTheme(session);
  // A saved search of the plan, so the page is called by its name, not *Search*.
  const saved = dataOf(
    await session.client.POST("/api/v1/saved-searches", {
      body: { name: plan.goal.title, search_text: planSearch },
    }),
  ).id;
  await session.dispose();
  const page = await samplePage(browser, server, ada);
  await openPlanAt(page, savedSearchAddress(saved), true);
  // The view of the timeline is kept with the saved search, as a person who plans with it does.
  await page.getByRole("button", { name: t("search.save-changes"), exact: true }).click();
  await expect(page.getByText(t("search.changed-not-saved"))).toBeHidden();
  await todayAtTheStart(page);
  await page.waitForLoadState("networkidle");
  await shootPicture(page, "timeline");
});
