import { dataOf } from "@bugaboo/api-client";

import type { ApiSession } from "./api.ts";
import { createIssue, versionOf, type MadeIssue } from "./issues.ts";

/** Gives an issue an estimate, in whole minutes. */
export async function estimate(session: ApiSession, issue: MadeIssue, minutes: number) {
  dataOf(
    await session.client.PATCH("/api/v1/issues/{issue}", {
      params: { path: { issue: issue.key } },
      body: { version: await versionOf(session, issue), estimated_time: minutes },
    }),
  );
}

/** Books minutes on an issue as the person of the session. */
export async function book(session: ApiSession, issue: MadeIssue, minutes: number) {
  dataOf(
    await session.client.POST("/api/v1/me/time-entries", {
      body: { issue: issue.key, date: "2027-01-14", minutes },
    }),
  );
}

/** Puts an issue under another. */
export async function putUnder(session: ApiSession, child: MadeIssue, parent: MadeIssue) {
  dataOf(
    await session.client.PUT("/api/v1/issues/{issue}/parent", {
      params: { path: { issue: child.key } },
      body: { version: await versionOf(session, child), parent: parent.key },
    }),
  );
}

/**
 * A goal estimated at 6 hours with 1 hour 20 minutes booked, and under it a task of 4 hours with 2
 * hours booked: the issue page shows its own numbers and the totals with the sub-issue.
 */
export async function goalWithTask(admin: ApiSession): Promise<MadeIssue> {
  const goal = await createIssue(admin, "WEB", "Ship the reset e-mail");
  const task = await createIssue(admin, "WEB", "Write the e-mail text");
  await putUnder(admin, task, goal);
  await estimate(admin, goal, 360);
  await estimate(admin, task, 240);
  await book(admin, goal, 80);
  await book(admin, task, 120);
  return goal;
}
