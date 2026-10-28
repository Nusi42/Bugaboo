import { type ApiSession } from "../../app/e2e/support/api.ts";
import { listPage } from "../../app/e2e/support/perf-reads.ts";
import { inOrder } from "../../app/e2e/support/sequence.ts";

/** How many projects the issues to read are taken from, and how many pages of each. */
const PROJECTS_READ = 8;
const PAGES_READ = 3;

/** The readable IDs of the pages of the list of one project, up to `PAGES_READ` pages. */
async function keysOf(
  session: ApiSession,
  project: string,
  cursor?: string,
  page = 0,
): Promise<readonly string[]> {
  const read = await listPage(session, project, cursor);
  const keys: string[] = [];
  for (const item of read.items) {
    if (typeof item.readable_id === "string") {
      keys.push(item.readable_id);
    }
  }
  const next = read.next_cursor ?? undefined;
  if (next === undefined || page + 1 >= PAGES_READ) {
    return keys;
  }
  return [...keys, ...(await keysOf(session, project, next, page + 1))];
}

/** The readable IDs of the first pages of the lists of some projects, spread over all of them. */
export async function someIssues(
  session: ApiSession,
  projects: readonly string[],
): Promise<readonly string[]> {
  const step = Math.max(1, Math.floor(projects.length / PROJECTS_READ));
  const chosen = projects.filter((_, at) => at % step === 0).slice(0, PROJECTS_READ);
  const keys: string[] = [];
  await inOrder(
    chosen.map((project) => async () => {
      keys.push(...(await keysOf(session, project)));
    }),
  );
  return keys;
}

/** The readable IDs of the first `count` issues of the list of a project. */
export async function firstIssues(
  session: ApiSession,
  project: string,
  count: number,
  cursor?: string,
  found: readonly string[] = [],
): Promise<readonly string[]> {
  const page = await listPage(session, project, cursor);
  const keys = [...found];
  for (const item of page.items) {
    if (typeof item.readable_id === "string") {
      keys.push(item.readable_id);
    }
  }
  const next = page.next_cursor ?? undefined;
  return next === undefined || keys.length >= count
    ? keys.slice(0, count)
    : firstIssues(session, project, count, next, keys);
}
