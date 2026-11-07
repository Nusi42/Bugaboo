import { dataOf } from "@bugaboo/api-client";

import { signInThroughApi } from "./accounts.ts";
import { ana, days } from "./demo-m6-common.ts";
import { createIssue } from "./issues.ts";
import { sampleAdministrator } from "./sample.ts";
import { type Instance } from "./server.ts";
import { choiceNamed, setTimeDefault, starterField } from "./time.ts";

/** The issue that was booked on in step 8, which step 9 reads as another person. */
let booked = "";

/** The key of the issue that was booked on. */
export function bookedIssue(): string {
  return booked;
}

/**
 * Makes an issue whose time defaults hold an activity and a cost center and books 45 minutes on it as
 * Ana, so the entry in its history has fields to say beside the sentence.
 */
export async function bookedByAna(server: Instance): Promise<string> {
  const admin = await signInThroughApi(server, sampleAdministrator());
  const issue = await createIssue(admin, "WEB", "Toner is low");
  const activity = await starterField(admin, "activity");
  const costCenter = await starterField(admin, "cost-center");
  await setTimeDefault(admin, issue, activity, await choiceNamed(admin, activity, "Testing"));
  await setTimeDefault(
    admin,
    issue,
    costCenter,
    await choiceNamed(admin, costCenter, "Product development", "4100"),
  );
  await admin.dispose();
  const session = await signInThroughApi(server, ana());
  dataOf(
    await session.client.POST("/api/v1/me/time-entries", {
      body: { issue: issue.key, date: days().today, minutes: 45 },
    }),
  );
  await session.dispose();
  booked = issue.key;
  return issue.key;
}
