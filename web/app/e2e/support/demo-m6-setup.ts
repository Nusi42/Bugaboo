import { dataOf } from "@bugaboo/api-client";

import { signInThroughApi } from "./accounts.ts";
import { type ApiSession } from "./api.ts";
import { days } from "./demo-m6-common.ts";
import { sampleAdministrator, sampleCostCenter } from "./sample.ts";
import { type Instance } from "./server.ts";
import { changeWorkDay } from "./timesheet.ts";
import { choiceNamed, earlierToday, starterField } from "./time.ts";

// What the exit demo of M6 (`m6-exit-demo.spec.ts`) sets up before a step, through the API.

/** How long ago the work day of Ana started: before the timer, so the time booked lies inside it. */
const DAY_MINUTES_AGO = 60;

/**
 * The day of Ana started `DAY_MINUTES_AGO` minutes ago, not at the tap: the timer is moved back by
 * its 25 minutes, and the time it books must lie inside the work day.
 */
export async function startedTheDayEarlier(session: ApiSession): Promise<void> {
  const start = earlierToday(DAY_MINUTES_AGO * 60);
  await changeWorkDay(session, days().today, [{ start, end: null }]);
}

/** The issue gets an Activity and a Cost center for the time booked on it, so the entry of a stop is filled in. */
export async function giveTheIssueItsDefaults(server: Instance, key: string): Promise<void> {
  const admin = await signInThroughApi(server, sampleAdministrator());
  const activity = await starterField(admin, "activity");
  const costCenter = await starterField(admin, "cost-center");
  const center = sampleCostCenter();
  const shown = dataOf(
    await admin.client.GET("/api/v1/issues/{issue}", { params: { path: { issue: key } } }),
  );
  dataOf(
    await admin.client.PATCH("/api/v1/issues/{issue}", {
      params: { path: { issue: key } },
      body: {
        version: shown.issue.version ?? 0,
        fields: {
          [activity]: await choiceNamed(admin, activity, "Testing"),
          [costCenter]: await choiceNamed(admin, costCenter, center.name, center.number),
        },
      },
    }),
  );
  await admin.dispose();
}
