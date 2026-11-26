import { t } from "@bugaboo/messages";
import { expect } from "@playwright/test";

import { listWord } from "../src/shared/list-columns.ts";
import { projectIssuesAddress } from "../src/shared/paths.ts";
import { ada, signInThroughApi } from "./support/accounts.ts";
import { createTask } from "./support/dashboard-issues.ts";
import { test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";

// The totals of a grouped list on a computer (QRY-08, M10-Q15): under each group a quiet row with its count and the
// sums of the columns of lengths, and a sum over fewer issues than the group holds says over how many. A phone has no
// columns: its row says the count.

test.use({ preparation: { run: prepareWeb } });

/** The row of totals of a group, by the words screen readers hear first. */
function totalsOf(page: Parameters<typeof signIn>[0], group: string) {
  return page.getByRole("row", { name: t("issues.totals-of", { group }), exact: false });
}

test("qry_08_a_grouped_list_shows_a_totals_row_per_group", async ({ page, server }) => {
  const admin = await signInThroughApi(server, ada);
  await createTask(admin, "Fix the header", { priority: "high", estimated_time: 60 });
  await createTask(admin, "Fix the footer", { priority: "high", estimated_time: 30 });
  await createTask(admin, "Write the guide", { priority: "low" });
  await createTask(admin, "Check the guide", { priority: "low", estimated_time: 120 });
  await admin.dispose();
  await signIn(page, ada);
  const columns = [listWord.status, listWord.id, listWord.title, listWord.estimatedTime];
  await page.goto(
    `${projectIssuesAddress("WEB")}?group=${listWord.priority}&cols=${encodeURIComponent(columns.join(","))}`,
  );
  const high = totalsOf(page, t("priority.high"));
  await expect(high).toBeVisible();
  await expect(high).toContainText(t("issues.totals-count", { count: 2 }));
  await expect(high).toContainText(t("issues.totals"));
  // 60 and 30 minutes: both issues gave a value.
  await expect(high).toContainText(/1h 30m$/u);
  // One of the two low ones has no estimate: the sum says over how many it is.
  const low = totalsOf(page, t("priority.low"));
  await expect(low).toContainText(t("issues.totals-count", { count: 2 }));
  await expect(low).toContainText(t("issues.totals-over", { total: "2h", valued: 1, issues: 2 }));
});
