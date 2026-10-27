import { t } from "@bugaboo/messages";

import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { grid } from "./support/list-look.ts";
import { partIn } from "./support/parts.ts";
import { sampleTeamMember } from "./support/sample.ts";
import { inOrder } from "./support/sequence.ts";

// Long lists (PERF-04, M5 WP-08): the rows that are not on screen are not drawn.

test.use({ sample: true });

test("perf_04 a list of ten thousand draws only the rows on screen", async ({ page }) => {
  test.setTimeout(180_000);
  await signIn(page, sampleTeamMember());
  await page.goto("/search?group=-");
  const rows = partIn(grid(page), "list-row");
  await expect(rows.first()).toBeVisible();
  expect(await rows.count()).toBeLessThan(100);
  // Scroll to the end of what is loaded, several times: more rows are read, none piles up.
  await inOrder(
    Array.from({ length: 4 }, () => async () => {
      await grid(page).evaluate((list) => {
        list.scrollTop = list.scrollHeight;
      });
      await expect.poll(() => rows.count()).toBeLessThan(100);
    }),
  );
  await expect(
    page.getByText(new RegExp(`${t("issues.everywhere")} · [1-9][0-9]{2,} `, "u")),
  ).toBeVisible();
});

test("perf_04 a grouped list of ten thousand draws only the rows on screen too", async ({
  page,
}) => {
  test.setTimeout(180_000);
  await signIn(page, sampleTeamMember());
  // The built-in list is grouped by status group: every group draws only what is on screen.
  await page.goto("/search");
  const rows = partIn(grid(page), "list-row");
  await expect(rows.first()).toBeVisible();
  expect(await rows.count()).toBeLessThan(100);
  await grid(page).evaluate((list) => {
    list.scrollTop = list.scrollHeight;
  });
  await expect.poll(() => rows.count()).toBeLessThan(100);
});
