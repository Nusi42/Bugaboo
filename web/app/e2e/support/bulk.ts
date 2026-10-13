import { dataOf } from "@bugaboo/api-client";

import { type ApiSession, type RawAnswer } from "./api.ts";
import { readIssue } from "./crowd-checks.ts";
import { issueOf, type MadeIssue } from "./issues.ts";

// Changing many issues at once (ISS-09, M3 WP-08). The route comes with WP-08, so this file is the
// one place that knows it: the scenario of ten people uses nothing else of it, and when the route
// has another shape, only `bulkBody` changes.

/** The route that changes many issues, as the API description names it. */
export interface BulkRoute {
  readonly path: string;
}

/** Whether the route is a POST whose name says *bulk*. */
function isBulk(path: string, operations: unknown): boolean {
  const post: unknown =
    typeof operations === "object" && operations !== null
      ? Reflect.get(operations, "post")
      : undefined;
  const id: unknown =
    typeof post === "object" && post !== null ? Reflect.get(post, "operationId") : "";
  return typeof id === "string" && /bulk/iu.test(`${id} ${path}`);
}

/** The route that changes many issues, when the server has one. */
export async function findBulkRoute(session: ApiSession): Promise<BulkRoute | undefined> {
  const description: unknown = dataOf(await session.client.GET("/api/v1/openapi.json"));
  const paths: unknown =
    typeof description === "object" && description !== null
      ? Reflect.get(description, "paths")
      : undefined;
  const all = typeof paths === "object" && paths !== null ? paths : {};
  const path = Object.keys(all).find((each) => isBulk(each, Reflect.get(all, each)));
  return path === undefined ? undefined : { path };
}

/** The route that changes many issues; fails when the server has none. */
export async function bulkRoute(session: ApiSession): Promise<BulkRoute> {
  const found = await findBulkRoute(session);
  if (found === undefined) {
    throw new TypeError("The server has no route that changes many issues.");
  }
  return found;
}

/** An issue with the version the caller read. */
export interface IssueAtVersion extends MadeIssue {
  readonly version: number;
}

/** An issue by its readable ID, with the version it has now. */
export async function atVersion(session: ApiSession, key: string): Promise<IssueAtVersion> {
  const made = await issueOf(session, key);
  return { ...made, version: (await readIssue(session, made)).issue.version ?? 1 };
}

/** The body that changes the status of the issues, each on the version the caller read. */
export function bulkBody(issues: readonly IssueAtVersion[], status: string): unknown {
  return {
    issues: issues.map((issue) => ({ issue: issue.id, version: issue.version })),
    changes: { status },
  };
}

/** Asks the server to change many issues at once. */
export function changeMany(
  session: ApiSession,
  route: BulkRoute,
  body: unknown,
): Promise<RawAnswer> {
  return session.post(route.path, body);
}

/** The status an issue has now. */
export async function statusOf(session: ApiSession, issue: MadeIssue): Promise<string | undefined> {
  return (await readIssue(session, issue)).issue.status ?? undefined;
}
