import { setTimeout as sleep } from "node:timers/promises";

import { dataOf } from "@bugaboo/api-client";
import { expect, type Page } from "@playwright/test";

import { createGroup, giveGroupRole } from "./access.ts";
import { createAdmin } from "./accounts.ts";
import { type ApiSession } from "./api.ts";
import { choiceBody, fieldOf } from "./fields.ts";
import { createProject } from "./projects.ts";
import { inOrder } from "./sequence.ts";
import { type Instance } from "./server.ts";

/** A date this many days before today, as the API writes it. */
export function daysAgo(count: number): string {
  const day = new Date();
  day.setUTCDate(day.getUTCDate() - count);
  return day.toISOString().slice(0, 10);
}

/** A bit more than a second: the pause the server asks between two changes of working hours. */
const PAUSE_MS = 1100;

/** The days a person has made in the past, for the page of the old work days: from the paper list. */
export async function createDays(session: ApiSession, dates: readonly string[]): Promise<void> {
  await inOrder(
    dates.map((date) => async () => {
      await sleep(PAUSE_MS);
      dataOf(
        await session.client.PATCH("/api/v1/me/work-days/{date}", {
          params: { path: { date } },
          body: {
            version: 0,
            periods: [{ start: `${date}T07:00:00Z`, end: `${date}T15:00:00Z` }],
            breaks: [],
            reason: "From the paper list",
          },
        }),
      );
    }),
  );
}

/** Adds a choice to the starter field of time entries with this name. */
export async function addChoice(
  admin: ApiSession,
  fieldName: string,
  name: string,
  number?: string,
) {
  const field = await fieldOf(admin, fieldName);
  dataOf(
    await admin.client.POST("/api/v1/fields/{field}/choices", {
      params: { path: { field: field.id } },
      body: choiceBody(field.version, name, number),
    }),
  );
}

/** Makes a choice of a starter field the default of a project, as its settings do. */
async function setDefault(admin: ApiSession, project: string, fieldName: string, choice: string) {
  const field = await fieldOf(admin, fieldName);
  const picked = field.choices.find((each) => each.name === choice);
  const shown = dataOf(
    await admin.client.GET("/api/v1/projects/{project}", { params: { path: { project } } }),
  );
  dataOf(
    await admin.client.PATCH("/api/v1/projects/{project}", {
      params: { path: { project: shown.project.id } },
      body: { version: shown.project.version, fields: { [field.id]: picked?.id ?? null } },
    }),
  );
}

/**
 * A server for the pictures of *Working hours*: the project WEB with a sub-project, a cost center
 * that the project WEB sets as its default, the group Payroll with the role HR, and two old work days.
 */
export async function prepareWorkingHours(server: Instance): Promise<void> {
  const admin = await createAdmin(server);
  const web = await createProject(admin, "WEB", "Website");
  await createProject(admin, "APP", "Mobile app", web);
  await addChoice(admin, "Cost center", "Product development", "4100");
  await setDefault(admin, "WEB", "Cost center", "Product development");
  await createGroup(admin, "Payroll").then((group) => giveGroupRole(admin, group, "HR", null));
  await createDays(admin, ["2020-03-02", "2020-03-03", daysAgo(10)]);
  await admin.dispose();
}

/** Presses a switch by its name, as a person does, and waits until it has the new state. */
export async function press(page: Page, name: string, on: boolean): Promise<void> {
  await page.getByText(name, { exact: true }).click();
  const field = page.getByRole("switch", { name, exact: true });
  await (on ? expect(field).toBeChecked() : expect(field).not.toBeChecked());
}
