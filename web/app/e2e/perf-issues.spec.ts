import { dataOf } from "@bugaboo/api-client";

import { signInThroughApi } from "./support/accounts.ts";
import { type ApiSession } from "./support/api.ts";
import { expect, test } from "./support/fixtures.ts";
import { BUDGET_MS, buildOf, timed, Times, writeNumbers } from "./support/perf.ts";
import { roleName } from "./support/access.ts";
import { sampleAdministrator, sampleEverywhere, sampleProjects } from "./support/sample.ts";
import { inOrder } from "./support/sequence.ts";

// Speed on the sample organization (PERF-01, M3 WP-11): the pages of an issue and the lists of
// issues answer within the budget at the 95th percentile, on the sample's ten thousand issues,
// with the program the tests run against (`BB_SERVER_BIN` names another build). The numbers are
// written to `test-results/perf-*.json` for the review of the milestone.

test.use({ sample: true });
test.describe.configure({ mode: "serial" });

/** How many issues are read as pages: spread over every project of the sample, new and old. */
const PAGES = 200;
/** How many times the list of each project is read. */
const LISTS = 6;
/** The size of a page of the list, as the screen asks for it. */
const PAGE_LEN = 50;
/** How many pages of a list are read one after the other, as a person scrolls. */
const PAGES_OF_A_LIST = 3;

/** The program the tests run against. */
const build = buildOf(process.env["BB_SERVER_BIN"]);

/** One page of the list of a project. */
async function listPage(session: ApiSession, project: string, cursor: string | undefined) {
  return dataOf(
    await session.client.GET("/api/v1/projects/{project}/issues", {
      params: {
        path: { project },
        query: { limit: PAGE_LEN, ...(cursor === undefined ? {} : { cursor }) },
      },
    }),
  );
}

/** The readable IDs of the first `count` issues of a project, newest first. */
async function firstIssues(
  session: ApiSession,
  project: string,
  count: number,
  cursor?: string,
  found: readonly string[] = [],
): Promise<readonly string[]> {
  const page = await listPage(session, project, cursor);
  const keys = [...found];
  for (const item of page.items) {
    if (typeof item.key === "string") {
      keys.push(item.key);
    }
  }
  const next = page.next_cursor ?? undefined;
  return next === undefined || keys.length >= count
    ? keys
    : firstIssues(session, project, count, next, keys);
}

/** The readable IDs of issues spread over the projects of the sample, old and new. */
async function someIssues(session: ApiSession, count: number): Promise<readonly string[]> {
  const perProject = Math.ceil((count * 3) / sampleProjects.length);
  const found = await Promise.all(
    sampleProjects.map((project) => firstIssues(session, project.shortName, perProject)),
  );
  // Every third, so the issues are spread over what was read.
  return found.flatMap((keys) => keys.filter((_, at) => at % 3 === 0)).slice(0, count);
}

/** One read of the issue page, asked for the way the browser asks: all at once. */
async function readPage(session: ApiSession, issue: string, times: Readonly<Times>): Promise<void> {
  const params = { path: { issue } };
  const read = async (kind: string, ask: () => Promise<unknown>): Promise<void> => {
    times.add(kind, (await timed(ask)).ms);
  };
  const whole = await timed(() =>
    Promise.all([
      read("issue", () => session.client.GET("/api/v1/issues/{issue}", { params })),
      read("relations", () => session.client.GET("/api/v1/issues/{issue}/relations", { params })),
      read("sub-issues", () => session.client.GET("/api/v1/issues/{issue}/sub-issues", { params })),
      read("related-issues", () =>
        session.client.GET("/api/v1/issues/{issue}/related-issues", { params }),
      ),
      read("history", () => session.client.GET("/api/v1/issues/{issue}/history", { params })),
      read("sorting", () => session.client.GET("/api/v1/issues/{issue}/sorting", { params })),
      read("follow", () => session.client.GET("/api/v1/issues/{issue}/follow", { params })),
      read("followers", () => session.client.GET("/api/v1/issues/{issue}/followers", { params })),
    ]),
  );
  times.add("the whole page, all reads at once", whole.ms);
}

/** Reads the first pages of the list of a project, as the list screen does when a person scrolls. */
async function readList(
  session: ApiSession,
  project: string,
  times: Readonly<Times>,
  page = 0,
  cursor?: string,
): Promise<void> {
  const read = await timed(() => listPage(session, project, cursor));
  times.add(page === 0 ? "list, first page" : "list, later page", read.ms);
  const next = read.value.next_cursor ?? undefined;
  if (next !== undefined && page + 1 < PAGES_OF_A_LIST) {
    await readList(session, project, times, page + 1, next);
  }
}

/** Says what is over the budget, with the numbers, so a failing run tells where. */
function expectInBudget(times: Readonly<Times>): void {
  expect(
    times.overBudget(),
    `the 95th percentile must be under ${String(BUDGET_MS)} ms: ${JSON.stringify(times.summaries())}`,
  ).toEqual([]);
}

test("perf_01_issue_page_and_list_stay_under_100_ms_p95_on_the_sample", async ({ server }) => {
  test.setTimeout(300_000);
  const session = await signInThroughApi(server, sampleAdministrator());
  const keys = await someIssues(session, PAGES);
  expect(keys.length).toBeGreaterThanOrEqual(PAGES / 2);
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

/** How many people write at the same time, and how many comments each of them writes. */
const WRITERS = 10;
const COMMENTS_EACH = 40;

/** A comment on an issue, timed. */
async function comment(
  session: ApiSession,
  issue: string,
  text: string,
  times: Readonly<Times>,
): Promise<void> {
  const done = await timed(() =>
    session.client.POST("/api/v1/issues/{issue}/comments", {
      params: { path: { issue } },
      body: { text },
    }),
  );
  times.add("write: a comment", done.ms);
}

/** What the project list and the preview of a move cost while the others write (M2-F26). */
async function readTree(
  session: ApiSession,
  moving: string,
  times: Readonly<Times>,
): Promise<void> {
  const list = await timed(() =>
    session.client.GET("/api/v1/projects", { params: { query: { limit: 100 } } }),
  );
  times.add("read: the list of projects", list.ms);
  const preview = await timed(() =>
    session.client.POST("/api/v1/projects/{project}/move-preview", {
      params: { path: { project: moving } },
      body: { to: null },
    }),
  );
  times.add("read: the preview of a move", preview.ms);
}

/** The ID of a project, found by its short name. */
async function idOfProject(session: ApiSession, shortName: string): Promise<string> {
  const found = dataOf(
    await session.client.GET("/api/v1/projects/{project}", {
      params: { path: { project: shortName } },
    }),
  );
  const id: unknown = found.project.id;
  if (typeof id !== "string") {
    throw new TypeError(`The project ${shortName} has no ID.`);
  }
  return id;
}

/** Everybody writes their comments, one after the other, on issues of the list. */
function writing(
  sessions: readonly ApiSession[],
  keys: readonly string[],
  times: Readonly<Times>,
): readonly Promise<void>[] {
  return sessions.map((session, who) =>
    inOrder(
      Array.from({ length: COMMENTS_EACH }, (_, at) => () => {
        const issue = keys[(who * COMMENTS_EACH + at) % keys.length] ?? "";
        const text = `Comment ${String(at + 1)} of person ${String(who + 1)}.`;
        return comment(session, issue, text, times);
      }),
    ),
  );
}

/** One person reads pages, the list of projects and the preview of a move, one after the other. */
function reading(
  session: ApiSession,
  keys: readonly string[],
  moving: string,
  times: Readonly<Times>,
): Promise<void> {
  return inOrder(
    keys.slice(0, 40).map((key) => async () => {
      await readPage(session, key, times);
      await readTree(session, moving, times);
    }),
  );
}

/** The first of a list that is not empty. */
function firstOf<T>(items: readonly T[]): T {
  const [first] = items;
  if (first === undefined) {
    throw new Error("There is nothing in the list.");
  }
  return first;
}

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
    reading(reader, keys, await idOfProject(reader, "IOS"), times),
  ]);
  writeNumbers("perf-01-ten-writers", build, times);
  expectInBudget(times);
  await Promise.all(sessions.map((session) => session.dispose()));
});

/** How long combining two choices of a field may hold the change, in milliseconds (M3-F60). */
const COMBINE_BUDGET_MS = 1000;

/** A field of the sample with its choices, found by name. */
async function fieldNamed(session: ApiSession, name: string) {
  const list = dataOf(await session.client.GET("/api/v1/fields"));
  for (const field of list.fields) {
    if (field.name === name) {
      return field;
    }
  }
  throw new Error(`The sample has no field called ${name}.`);
}

/** The ID of the choice with this name among the choices of a field. */
function choiceNamed(
  choices: readonly { readonly id: string; readonly name: string }[],
  name: string,
): string {
  for (const choice of choices) {
    if (choice.name === name) {
      return choice.id;
    }
  }
  throw new Error(`There is no choice called ${name}.`);
}

test("perf_f60_combining_a_choice_with_another_on_the_sample_holds_the_change_under_a_second", async ({
  server,
}) => {
  const session = await signInThroughApi(server, sampleAdministrator());
  const severity = await fieldNamed(session, "Severity");
  const combined = await timed(() =>
    session.client.POST("/api/v1/fields/{field}/choices/{choice}/combine", {
      params: { path: { field: severity.id, choice: choiceNamed(severity.choices, "Minor") } },
      body: { into: choiceNamed(severity.choices, "Major"), version: severity.version },
    }),
  );
  const times = new Times();
  times.add("combine: one choice into another, on the sample", combined.ms);
  writeNumbers("perf-f60-combine", build, times);
  expect(combined.ms).toBeLessThan(COMBINE_BUDGET_MS);
  await session.dispose();
});
