import { dataOf } from "@bugaboo/api-client";
import { expect, type Page } from "@playwright/test";

import layout from "../../../../tools/dev/sample.json" with { type: "json" };
import { personAddress } from "../../src/shared/paths.ts";
import { signInThroughApi, type ApiSession, type Instance } from "./accounts.ts";
import { goodWindowInFourThemes } from "./demo.ts";
import { sampleNamed, type SamplePerson } from "./sample.ts";

// What the steps of the exit demo of M9 share (`m9-exit-demo.spec.ts`): the pictures and the people of
// the sample organization the steps name.

/** How long a step of the demo may take: a step signs in, works and takes its pictures in four themes. */
export const STEP_TIMEOUT_MS = 300_000;

/** The pictures of the demo, in the folder of the screenshots. */
const DEMO_M9 = "M9/exit-demo";

/** The part of the sample about the people who left, the export and the backups (part 8). */
export const privacy = layout.issues.work.privacy;

/** The screen is good in the four themes and is in the pictures of this demo, as the window shows it. */
export function good(page: Page, step: string): Promise<void> {
  return goodWindowInFourThemes(page, step, DEMO_M9);
}

/** Ada Brandt, the first Administrator of the sample. */
export function ada(): SamplePerson {
  return sampleNamed("Ada Brandt");
}

/** Ben Hale, whom the demo switches off, whose data an administrator downloads and who is removed. */
export function ben(): SamplePerson {
  return sampleNamed("Ben Hale");
}

/** Lea Fischer, who downloads her own data (her file is ready in the sample). */
export function lea(): SamplePerson {
  return sampleNamed(privacy.exportOf);
}

/** Noah Weber, who saves issues while the backup is made. */
export function noah(): SamplePerson {
  return sampleNamed("Noah Weber");
}

/** Hana Weiss, who holds *HR* and sees the work days of people. */
export function hana(): SamplePerson {
  return sampleNamed("Hana Weiss");
}

/** Ada, signed in at the other end of the API. */
export function adaThroughApi(server: Instance): Promise<ApiSession> {
  return signInThroughApi(server, ada());
}

/** The ID of a person of the sample, from the list of people as an administrator reads it. */
export async function idOfPerson(admin: ApiSession, person: SamplePerson): Promise<string> {
  const found = dataOf(
    await admin.client.GET("/api/v1/people", { params: { query: { search: person.email } } }),
  ).items[0]?.person;
  if (found?.shown !== "name") {
    throw new TypeError(`${person.email} is not in the list of people.`);
  }
  return found.id;
}

/** Opens the page of a person of the sample as the administrator who is signed in on the page. */
export async function openPersonOf(page: Page, id: string, name: string): Promise<void> {
  await page.goto(personAddress(id));
  await expect(page.getByRole("heading", { level: 1, name })).toBeVisible();
}
