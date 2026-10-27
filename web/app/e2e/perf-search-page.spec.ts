import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { buildOf, Times, writeNumbers } from "./support/perf.ts";
import { expectInBudget } from "./support/perf-reads.ts";
import { SEARCH_PAGE_REQUESTS, timeSearchPage } from "./support/perf-search.ts";
import { sampleTeamMember } from "./support/sample.ts";
import { inOrder } from "./support/sequence.ts";

// The first page of the Search page on the sample (PERF-01, M5 WP-08): the three requests the page
// makes when it opens (the first page of issues with the fields of its columns, the count for the
// count line, and the groups of the default grouping), each within the budget at the 95th
// percentile, on the sample's ten thousand issues. The page itself asks, so what is timed is what
// a person's browser asks for.

test.use({ sample: true });

/** How many times the page is opened. */
const OPENINGS = 20;

test("perf_01_the_first_page_of_the_search_page_stays_under_100_ms_p95_on_the_sample", async ({
  page,
}) => {
  test.setTimeout(300_000);
  await signIn(page, sampleTeamMember());
  const times = new Times();
  timeSearchPage(page, times);
  await inOrder(
    Array.from({ length: OPENINGS }, () => async () => {
      await page.goto("/search");
      await page.waitForLoadState("networkidle");
    }),
  );
  writeNumbers("perf-01-search-page", buildOf(process.env["BB_SERVER_BIN"]), times);
  expect(Object.keys(times.summaries())).toHaveLength(SEARCH_PAGE_REQUESTS);
  expectInBudget(times);
});
