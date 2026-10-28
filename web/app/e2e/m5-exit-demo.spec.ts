import { dataOf } from "@bugaboo/api-client";

import {
  aHiddenFieldIsAnUnknownField,
  sharedSearchesReachWhoReadThem,
  simpleSearchFindsIssues,
  switchingKeepsTheSearch,
  theProjectShowsItsList,
  good,
} from "./support/demo-m5.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { ask, result } from "./support/quick-search.ts";
import { sampleAdministrator, sampleTeamMember } from "./support/sample.ts";
import { signInThroughApi } from "./support/accounts.ts";

// The exit demo of M5 (docs/04-milestones.md) on the sample organization, as the maintainer can repeat
// it by hand with `cargo xtask dev --sample`: search the sample in both modes and switch between
// them without losing the search; a Contributor who may not read *Customer* searches by it and
// gets the answer of a field that is not there; the searches people of the sample saved reach the
// people they are shared with, and none that reads *Customer* reaches a Contributor; a project
// shows the search chosen as its list; Quick search finds an issue and opens it. Every step is good
// in the four themes on a phone and on a computer, and with `BB_SCREENSHOT_DIR` set the same run
// takes the pictures of the review page (`<folder>/M5/demo/<step>-<phone|computer>-<theme>.png`).
// The steps are in `support/demo-m5.ts`. That the run on 1 M issues meets its budget is the
// nightly benchmark (`cargo xtask bench`, PERF-BENCH).

test.use({ sample: true });
test.describe.configure({ mode: "serial" });

test("1. Simple search finds issues of the sample", async ({ page }) => {
  test.setTimeout(180_000);
  await simpleSearchFindsIssues(page);
});

test("2. Advanced search shows the same search and switching loses nothing", async ({ page }) => {
  test.setTimeout(180_000);
  await switchingKeepsTheSearch(page);
});

test("3. A Contributor who may not read Customer gets the answer of a field that is not there", async ({
  page,
  server,
}) => {
  test.setTimeout(180_000);
  await aHiddenFieldIsAnUnknownField(page, server);
});

test("4. Shared searches reach the people who read them, and none that reads Customer a Contributor", async ({
  page,
  server,
}) => {
  test.setTimeout(180_000);
  await sharedSearchesReachWhoReadThem(page, server);
});

test("5. A project shows the search chosen as its list", async ({ page }) => {
  test.setTimeout(180_000);
  await theProjectShowsItsList(page);
});

test("6. Quick search finds an issue of the sample and opens it", async ({ page, server }) => {
  test.setTimeout(180_000);
  const admin = await signInThroughApi(server, sampleAdministrator());
  const found = dataOf(
    await admin.client.GET("/api/v1/search/issues", {
      params: { query: { q: "project is WEB", limit: 1, fields: "title" } },
    }),
  );
  await admin.dispose();
  const title = String(found.items[0]?.title);
  await signIn(page, sampleTeamMember());
  await page.goto("/");
  await page.waitForLoadState("networkidle");
  await ask(page, title);
  await result(page, title).first().waitFor();
  await good(page, "06-quick-search");
  await page.keyboard.press("Enter");
  await expect(page.getByRole("heading", { name: title }).first()).toBeVisible();
});
