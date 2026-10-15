import { signInThroughApi } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { buildOf, timed, Times, writeNumbers } from "./support/perf.ts";
import { roleName } from "./support/access.ts";
import {
  sampleAdministrator,
  sampleCombine,
  sampleEverywhere,
  sampleProjects,
  sampleSubProject,
} from "./support/sample.ts";
import { inOrder } from "./support/sequence.ts";
import {
  choiceNamed,
  comment,
  expectInBudget,
  fieldNamed,
  firstOf,
  idOfProject,
  LISTS,
  PAGES,
  readList,
  readPage,
  reading,
  someIssues,
  WRITERS,
  writing,
} from "./support/perf-reads.ts";

// Speed on the sample organization (PERF-01, M3 WP-11): the pages of an issue and the lists of
// issues answer within the budget at the 95th percentile, on the sample's ten thousand issues,
// with the program the tests run against (`BB_SERVER_BIN` names another build). The numbers are
// written to `test-results/perf-*.json` for the review of the milestone.

test.use({ sample: true });
test.describe.configure({ mode: "serial" });

/** The program the tests run against. */
const build = buildOf(process.env["BB_SERVER_BIN"]);

test("perf_01_issue_page_and_list_stay_under_100_ms_p95_on_the_sample", async ({ server }) => {
  test.setTimeout(300_000);
  const session = await signInThroughApi(server, sampleAdministrator());
  const keys = await someIssues(session, PAGES);
  expect(keys.length).toBe(PAGES);
  // The first reads warm the statements and the caches of the program; they are not counted.
  const warm = new Times();
  await inOrder(keys.slice(0, 10).map((key) => () => readPage(session, key, warm)));

  const times = new Times();
  await inOrder(keys.map((key) => () => readPage(session, key, times)));
  await inOrder(
    Array.from(
      { length: LISTS },
      () => () =>
        inOrder(sampleProjects.map((project) => () => readList(session, project.shortName, times))),
    ),
  );
  writeNumbers("perf-01-issue-page-and-list", build, times);
  expectInBudget(times);
  await session.dispose();
});

test("perf_01_ten_writers_and_a_reader_stay_under_100_ms_p95_on_the_sample", async ({ server }) => {
  test.setTimeout(300_000);
  const people = [sampleAdministrator(), ...sampleEverywhere(roleName.teamMember, WRITERS - 1)];
  const sessions = await Promise.all(people.map((person) => signInThroughApi(server, person)));
  const reader = firstOf(sessions);
  const keys = await someIssues(reader, PAGES);
  const times = new Times();
  // Warm up, then everybody at once: ten people write comments while one reads pages, the list of
  // projects and the preview of a move.
  await comment(reader, firstOf(keys), "Warming up.", new Times());
  await Promise.all([
    ...writing(sessions, keys, times),
    reading(reader, keys, await idOfProject(reader, sampleSubProject.shortName), times),
  ]);
  writeNumbers("perf-01-ten-writers", build, times);
  expectInBudget(times);
  await Promise.all(sessions.map((session) => session.dispose()));
});

/** How long combining two choices of a field may hold the change, in milliseconds (M3-F60). */
const COMBINE_BUDGET_MS = 1000;

test("perf_f60_combining_a_choice_with_another_on_the_sample_holds_the_change_under_a_second", async ({
  server,
}) => {
  const session = await signInThroughApi(server, sampleAdministrator());
  const severity = await fieldNamed(session, sampleCombine.field);
  const combined = await timed(() =>
    session.client.POST("/api/v1/fields/{field}/choices/{choice}/combine", {
      params: {
        path: { field: severity.id, choice: choiceNamed(severity.choices, sampleCombine.from) },
      },
      body: { into: choiceNamed(severity.choices, sampleCombine.into), version: severity.version },
    }),
  );
  const times = new Times();
  times.add("combine: one choice into another, on the sample", combined.ms);
  writeNumbers("perf-f60-combine", build, times);
  expect(combined.ms).toBeLessThan(COMBINE_BUDGET_MS);
  await session.dispose();
});
