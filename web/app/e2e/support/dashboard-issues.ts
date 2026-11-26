import { dataOf, type Schema } from "@bugaboo/api-client";
import { expect } from "@playwright/test";

import { dashboardAddress } from "../../src/shared/places.ts";

export { dashboardAddress } from "../../src/shared/places.ts";
import { ada, signInThroughApi, type ApiSession, type Instance } from "./accounts.ts";
import { createDashboard, type AskedBox } from "./dashboards.ts";
import { firstIssueType, taskIssueType } from "./issues.ts";

/** What a test sets on a new issue of WEB besides its title. */
interface Extra {
  readonly assigned_to?: readonly string[];
  readonly priority?: Schema<"Priority">;
  readonly estimated_time?: number;
}

/** Creates a task of WEB with a title and what the test sets on it. */
export async function createTask(api: ApiSession, title: string, extra: Extra = {}): Promise<void> {
  dataOf(
    await api.client.POST("/api/v1/projects/{project}/issues", {
      params: { path: { project: "WEB" } },
      body: {
        issue_type: await taskIssueType(api),
        title,
        ...(extra.priority === undefined ? {} : { priority: extra.priority }),
        ...(extra.estimated_time === undefined ? {} : { estimated_time: extra.estimated_time }),
        ...(extra.assigned_to === undefined ? {} : { assigned_to: [...extra.assigned_to] }),
      },
    }),
  );
}

/** Creates a bug of WEB with a priority. */
export async function createBug(
  api: ApiSession,
  title: string,
  priority: Schema<"Priority">,
): Promise<void> {
  dataOf(
    await api.client.POST("/api/v1/projects/{project}/issues", {
      params: { path: { project: "WEB" } },
      body: { issue_type: await firstIssueType(api), title, priority },
    }),
  );
}

/** A ready-made box as a new box of a dashboard asks for it. */
export async function readyMadeBox(api: ApiSession, which: Schema<"ReadyMade">): Promise<AskedBox> {
  const ready = dataOf(await api.client.GET("/api/v1/dashboards/ready-made-boxes"));
  const found = ready.boxes.find((each) => each.which === which);
  if (found === undefined) {
    throw new TypeError(`There is no ready-made box ${which}.`);
  }
  const { id: _id, ...asked } = found.box;
  return asked;
}

/** A dashboard of six ready-made boxes, two to a row, over two estimated tasks, as a person would build one; its ID. */
export async function createTeamBoard(api: ApiSession): Promise<string> {
  await createTask(api, "Fix the header", { priority: "high", estimated_time: 90 });
  await createTask(api, "Write the guide", { priority: "low", estimated_time: 240 });
  const boxes = await Promise.all(
    (
      [
        "open-bugs-by-priority",
        "issues-by-status-group",
        "my-open-issues",
        "time-on-this-search",
        "recent-changes-in-this-project",
        "my-timer",
      ] as const
    ).map((which) => readyMadeBox(api, which)),
  );
  const placed = boxes.map((box, at) =>
    Object.assign(box, { column: (at % 2) * 6, row: Math.floor(at / 2) * 4 }),
  );
  return (await createDashboard(api, "Website this week", placed)).id;
}

/** The address of a dashboard of six ready-made boxes of the administrator (see `createTeamBoard`). */
export async function teamBoardAddress(server: Instance): Promise<string> {
  const admin = await signInThroughApi(server, ada);
  const id = await createTeamBoard(admin);
  await admin.dispose();
  return dashboardAddress(id);
}

/** How many issues a search finds for the session now. */
async function foundBy(api: ApiSession, text: string): Promise<number> {
  const found = dataOf(
    await api.client.GET("/api/v1/search/issues", { params: { query: { q: text, limit: 50 } } }),
  );
  return found.items.length;
}

/**
 * Waits until the search finds the issues a test has just made: the index of the words of titles takes in a new issue
 * a moment after it is made, so a box read at once would find none of them.
 */
export async function untilSearchFinds(
  api: ApiSession,
  text: string,
  count: number,
): Promise<void> {
  await expect.poll(() => foundBy(api, text)).toBe(count);
}
