import { pageSizeMax } from "@bugaboo/api-client";

import { type ApiSession } from "../../app/e2e/support/api.ts";
import { previewStatus } from "../../app/e2e/support/many-changes.ts";
import { listPage, pageReads } from "../../app/e2e/support/perf-reads.ts";
import { NOTIFICATIONS_PAGE } from "../../app/e2e/support/perf-work.ts";
import { timed } from "../../app/e2e/support/perf.ts";
import { type BudgetName } from "./budgets.generated.ts";

// The scenarios of the benchmarks (PERF-01, architecture §11, "Large dataset"): registered here and
// nowhere else, so the packages that add a search or a list add their scenario to this list. Every
// scenario names its budget, which `tools/checks/perf-bench.toml` defines (PERF-BENCH checks that
// it does). `once` asks for what a screen asks for and says how long it took, in milliseconds.

/** What the scenarios read from: a signed-in person, and where to look. */
export interface Context {
  readonly session: ApiSession;
  /** The readable IDs of issues spread over several projects. */
  readonly issues: readonly string[];
  /** A project with very many issues, and one of the tail. */
  readonly big: string;
  readonly small: string;
  /** The readable IDs of the most issues one change to many issues names, from the big project. */
  readonly many: readonly string[];
}

/** One kind of read, repeated `runs` times after the warm-up. */
export interface Scenario {
  readonly name: string;
  readonly budget: BudgetName;
  readonly runs: number;
  /** How many runs are thrown away first; the usual number where it is not given. */
  readonly warmUp?: number;
  readonly once: (context: Context, round: number) => Promise<number>;
}

const RUNS = 200;
/** A preview of 5,000 issues takes seconds, so it is run fewer times than a read of a page. */
const PREVIEW_RUNS = 20;
const PREVIEW_WARM_UP = 3;

/** Asks for the reads of an issue page all at once, as the browser does, and times them together. */
async function issuePage(context: Context, round: number): Promise<number> {
  const issue = context.issues[round % context.issues.length] ?? "";
  const reads = pageReads(context.session, issue);
  return (await timed(() => Promise.all(reads.map(([, ask]) => ask())))).ms;
}

/** The first page of the list of a project. */
async function firstPage(context: Context, project: string): Promise<number> {
  return (await timed(() => listPage(context.session, project))).ms;
}

/** The number on the menu. */
async function unreadCount(context: Context): Promise<number> {
  return (await timed(() => context.session.client.GET("/api/v1/notifications/unread"))).ms;
}

/** The first page of the notifications. */
async function notificationsPage(context: Context): Promise<number> {
  return (
    await timed(() =>
      context.session.client.GET("/api/v1/notifications", {
        params: { query: { limit: NOTIFICATIONS_PAGE } },
      }),
    )
  ).ms;
}

/** The first page of a search across every project the person sees. */
async function searchPage(context: Context, q: string): Promise<number> {
  return (
    await timed(() =>
      context.session.client.GET("/api/v1/search/issues", { params: { query: { q, limit: 40 } } }),
    )
  ).ms;
}

/** How many issues a search finds. */
async function searchCount(context: Context, q: string): Promise<number> {
  return (
    await timed(() =>
      context.session.client.GET("/api/v1/search/issues/count", { params: { query: { q } } }),
    )
  ).ms;
}

/** The groups of a search by one field. */
async function searchGroups(context: Context, by: string, q: string): Promise<number> {
  return (
    await timed(() =>
      context.session.client.GET("/api/v1/search/issues/groups", { params: { query: { q, by } } }),
    )
  ).ms;
}

/** The preview of a change to the most issues one change names (M3-F133). */
async function previewMany(context: Context): Promise<number> {
  return (await timed(() => previewStatus(context.session, context.many, "done"))).ms;
}

/** The complex search of architecture section 8, across 500 projects. */
const COMPLEX = "statusGroup is not Done and priority is high";

export const scenarios: readonly Scenario[] = [
  {
    name: "issue-page",
    budget: "page",
    runs: RUNS,
    once: issuePage,
  },
  {
    name: "list-big-project",
    budget: "page",
    runs: RUNS,
    once: (context) => firstPage(context, context.big),
  },
  {
    name: "list-small-project",
    budget: "page",
    runs: RUNS,
    once: (context) => firstPage(context, context.small),
  },
  {
    name: "notifications-count",
    budget: "page",
    runs: RUNS,
    once: (context) => unreadCount(context),
  },
  {
    name: "notifications-page",
    budget: "page",
    runs: RUNS,
    once: (context) => notificationsPage(context),
  },
  {
    name: "project-list",
    budget: "page",
    runs: RUNS,
    once: async (context) =>
      (
        await timed(() =>
          context.session.client.GET("/api/v1/projects", {
            params: { query: { limit: pageSizeMax } },
          }),
        )
      ).ms,
  },
  {
    name: "search-complex",
    budget: "search",
    runs: RUNS,
    once: (context) => searchPage(context, COMPLEX),
  },
  {
    name: "search-complex-sorted",
    budget: "search",
    runs: RUNS,
    once: (context) => searchPage(context, `${COMPLEX} sort by created, oldest first`),
  },
  {
    name: "search-count",
    budget: "search",
    runs: RUNS,
    once: (context) => searchCount(context, COMPLEX),
  },
  {
    name: "search-groups-status",
    budget: "search",
    runs: RUNS,
    once: (context) => searchGroups(context, "status", "priority is high"),
  },
  {
    name: "search-groups-assigned-to",
    budget: "search",
    runs: RUNS,
    once: (context) => searchGroups(context, "assignedTo", COMPLEX),
  },
  // The preview judges each issue of the selection, so its cost grows with the issues (M3-F133).
  {
    name: "many-changes-preview",
    budget: "preview",
    runs: PREVIEW_RUNS,
    warmUp: PREVIEW_WARM_UP,
    once: previewMany,
  },
  // The text search (M5-F50): a word in very many issues, a word in few, and the start of a word.
  {
    name: "search-text-common-word",
    budget: "search",
    runs: RUNS,
    once: (context) => searchPage(context, 'contains "wrong"'),
  },
  {
    name: "search-text-rare-word",
    budget: "search",
    runs: RUNS,
    once: (context) => searchPage(context, 'contains "barcode"'),
  },
  {
    name: "search-text-prefix",
    budget: "search",
    runs: RUNS,
    once: (context) => searchPage(context, 'contains "notif*"'),
  },
  {
    name: "search-text-and-filter",
    budget: "search",
    runs: RUNS,
    once: (context) => searchPage(context, `contains "wrong" and ${COMPLEX}`),
  },
];
