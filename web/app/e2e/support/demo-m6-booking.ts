import { dataOf } from "@bugaboo/api-client";

import { signInThroughApi } from "./accounts.ts";
import { ana, days } from "./demo-m6-common.ts";
import { createIssue } from "./issues.ts";
import { sampleAdministrator, type SamplePerson } from "./sample.ts";
import { type Instance } from "./server.ts";
import { clearTimer, giveDefaults } from "./time.ts";

/** The issue that was booked on in step 8, which step 9 reads as another person. */
let booked = "";

/** The key of the issue that was booked on, booked on now when no step before did it (a step run alone). */
export function bookedIssueOf(server: Instance): Promise<string> {
  return booked === "" ? bookedByAna(server) : Promise.resolve(booked);
}

/**
 * Makes an issue whose time defaults hold an activity and a cost center and books 45 minutes on it as
 * Ana, so the entry in its history has fields to say beside the sentence.
 */
export async function bookedByAna(server: Instance): Promise<string> {
  const admin = await signInThroughApi(server, sampleAdministrator());
  const issue = await createIssue(admin, "WEB", "Toner is low");
  await giveDefaults(admin, issue);
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

/** Throws away the running timer of a person, whose toast would cover the history. */
export async function withoutTimer(server: Instance, person: SamplePerson): Promise<void> {
  const session = await signInThroughApi(server, person);
  await clearTimer(session);
  await session.dispose();
}
