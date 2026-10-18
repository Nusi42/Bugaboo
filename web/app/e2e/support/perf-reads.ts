import { dataOf, pageSizeDefault, pageSizeMax } from "@bugaboo/api-client";
import { expect } from "@playwright/test";

import { type ApiSession } from "./api.ts";
import { BUDGET_MS, timed, type Times } from "./perf.ts";
import { sampleProjects } from "./sample.ts";
import { inOrder } from "./sequence.ts";

// What the speed tests of the sample read and write (`perf-issues.spec.ts`).

/** How many issues are read as pages: spread over every project of the sample, new and old. */
export const PAGES = 200;
/** How many times the list of each project is read. */
export const LISTS = 6;
/** The size of a page of the list, as the screen asks for it (the usual page of the server). */
export const PAGE_LEN = pageSizeDefault;
/** How many pages of a list are read one after the other, as a person scrolls. */
export const PAGES_OF_A_LIST = 3;

/** One page of the list of a project. */
export async function listPage(session: ApiSession, project: string, cursor: string | undefined) {
  return dataOf(
    await session.client.GET("/api/v1/projects/{project}/issues", {
      params: {
        path: { project },
        query: { limit: PAGE_LEN, ...(cursor === undefined ? {} : { cursor }) },
      },
    }),
  );
}

/** The readable IDs of every issue of a project, newest first. */
export async function allIssues(
  session: ApiSession,
  project: string,
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
  return next === undefined ? keys : allIssues(session, project, next, keys);
}

/** Every `step`-th of the keys, so what is read is spread over the whole list, old and new. */
export function spread(keys: readonly string[], wanted: number): readonly string[] {
  const step = Math.max(1, Math.floor(keys.length / wanted));
  return keys.filter((_, at) => at % step === 0).slice(0, wanted);
}

/** The readable IDs of `count` issues spread over the whole list of every project of the sample. */
export async function someIssues(session: ApiSession, count: number): Promise<readonly string[]> {
  const perProject = Math.ceil(count / sampleProjects.length);
  const found = await Promise.all(
    sampleProjects.map(async (project) =>
      spread(await allIssues(session, project.shortName), perProject),
    ),
  );
  return found.flat().slice(0, count);
}

/** One kind of read of the issue page: its name, and the request. */
export type PageRead = readonly [kind: string, ask: () => Promise<unknown>];

/** The reads of the issue page: the issue, its relations, sub-issues, history, labels and followers. */
export function pageReads(session: ApiSession, issue: string): readonly PageRead[] {
  const params = { path: { issue } };
  return [
    ["issue", () => session.client.GET("/api/v1/issues/{issue}", { params })],
    ["relations", () => session.client.GET("/api/v1/issues/{issue}/relations", { params })],
    ["sub-issues", () => session.client.GET("/api/v1/issues/{issue}/sub-issues", { params })],
    [
      "related-issues",
      () => session.client.GET("/api/v1/issues/{issue}/related-issues", { params }),
    ],
    ["history", () => session.client.GET("/api/v1/issues/{issue}/history", { params })],
    [
      "labels-and-tags",
      () => session.client.GET("/api/v1/issues/{issue}/labels-and-tags", { params }),
    ],
    ["follow", () => session.client.GET("/api/v1/issues/{issue}/follow", { params })],
    ["followers", () => session.client.GET("/api/v1/issues/{issue}/followers", { params })],
  ];
}

/** Asks for all the reads at once, as the browser asks, and times each and the whole. */
export async function readAll(
  reads: readonly PageRead[],
  whole: string,
  times: Readonly<Times>,
): Promise<void> {
  const all = await timed(() =>
    Promise.all(
      reads.map(async ([kind, ask]) => {
        times.add(kind, (await timed(ask)).ms);
      }),
    ),
  );
  times.add(whole, all.ms);
}

/** One read of the issue page, asked for the way the browser asks: all at once. */
export async function readPage(
  session: ApiSession,
  issue: string,
  times: Readonly<Times>,
): Promise<void> {
  await readAll(pageReads(session, issue), "the whole page, all reads at once", times);
}

/** Reads the first pages of the list of a project, as the list screen does when a person scrolls. */
export async function readList(
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
export function expectInBudget(times: Readonly<Times>): void {
  expect(
    times.overBudget(),
    `the 95th percentile must be under ${String(BUDGET_MS)} ms: ${JSON.stringify(times.summaries())}`,
  ).toEqual([]);
}

/** How many people write at the same time, and how many comments each of them writes. */
export const WRITERS = 10;
export const COMMENTS_EACH = 40;

/** A comment on an issue, timed. */
export async function comment(
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
export async function readTree(
  session: ApiSession,
  moving: string,
  times: Readonly<Times>,
): Promise<void> {
  const list = await timed(() =>
    session.client.GET("/api/v1/projects", { params: { query: { limit: pageSizeMax } } }),
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
export async function idOfProject(session: ApiSession, shortName: string): Promise<string> {
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
export function writing(
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
export function reading(
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
export function firstOf<T>(items: readonly T[]): T {
  const [first] = items;
  if (first === undefined) {
    throw new Error("There is nothing in the list.");
  }
  return first;
}

/** The ID of the choice with this name among the choices of a field. */
export function choiceNamed(
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
