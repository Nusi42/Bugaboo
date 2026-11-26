import { mapIssuesMost } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";

import { ada, signInThroughApi, type Instance } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { createIssue, prepareWeb } from "./support/issues.ts";
import { kindWords, openMap, relate } from "./support/relation-map.ts";
import { inOrder } from "./support/sequence.ts";

// A crowd on the relation map (ISS-16, UX §4.15): a map that is cut at the most issues it holds says
// so and suggests fewer steps away or fewer kinds.

/** How many issues block each of the two hubs: more than a map holds, fewer than an issue's relations. */
const crowd = 120;

/** WEB-1 and WEB-4 each blocked by a crowd of issues: together more than a map holds. */
async function prepareCrowd(server: Instance): Promise<void> {
  await prepareWeb(server);
  const admin = await signInThroughApi(server, ada);
  const hub = await createIssue(admin, "WEB", "The second hub");
  await relate(admin, "WEB-1", kindWords.blocks, hub.key);
  await inOrder(
    Array.from({ length: crowd * 2 }, (_, at) => async () => {
      const made = await createIssue(admin, "WEB", `In the crowd ${String(at).padStart(3, "0")}`);
      await relate(admin, made.key, kindWords.blocks, at < crowd ? "WEB-1" : hub.key);
    }),
  );
  await admin.dispose();
}

test.use({ preparation: { run: prepareCrowd } });

test("iss_16_over_200_issues_the_map_says_so", async ({ page }) => {
  await signIn(page, ada);
  await openMap(page, "WEB-1");
  await expect(
    page.getByText(t("relation-map.too-many", { most: mapIssuesMost }), { exact: true }),
  ).toBeVisible();
  await expect(page.locator('[data-part="map-card"]')).toHaveCount(mapIssuesMost);
});

test("iss_16_a_search_with_more_results_than_a_map_holds_says_so", async ({ page }) => {
  await signIn(page, ada);
  await page.goto("/search?s=project%20is%20WEB&show=map");
  await expect(
    page.getByText(t("relation-map.too-many-found", { most: mapIssuesMost }), { exact: true }),
  ).toBeVisible();
  await expect(page.locator('[data-part="map-card"]')).toHaveCount(mapIssuesMost);
});
