import { dataOf, type Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { expect, type Locator, type Page } from "@playwright/test";

import { personWorkTimesAddress, paths } from "../../src/shared/paths.ts";
import { ada, cleo, ownId, signInThroughApi, type Person } from "./accounts.ts";
import { createPeopleWithRoles, projectIdOf, roleName } from "./access.ts";
import { prepareWeb } from "./issues.ts";
import { createProject } from "./projects.ts";
import { type Instance } from "./server.ts";

// The scene of the tests of work times (PLAN-11, PLAN-15): the project WEB of the tests of issues,
// with Bob as a Team member, Lea as a Guest and Cleo as its Project lead, and a second project, OTHER,
// where Bob is a Team member too and Cleo has no role, so she may not see it.

/** Gets the server ready: WEB with its people, Cleo as Project lead of WEB, and OTHER with Bob in it. */
export async function prepareWorkTimes(server: Instance): Promise<void> {
  await prepareWeb(server);
  const admin = await signInThroughApi(server, ada);
  await createProject(admin, "OTHER", "Other work");
  await createPeopleWithRoles(server, admin, [
    { person: cleo, role: roleName.projectLead, project: "WEB" },
  ]);
  await admin.dispose();
}

/** A day on the calendar `days` after today (`2027-01-14`): the page lists the coming year. */
export function dayFromToday(days: number): string {
  const day = new Date();
  day.setUTCDate(day.getUTCDate() + days);
  return day.toISOString().slice(0, "yyyy-mm-dd".length);
}

/** The ID of a person, through the API as the administrator sees it. */
export async function idOf(server: Instance, person: Person): Promise<string> {
  const session = await signInThroughApi(server, person);
  const id = await ownId(session);
  await session.dispose();
  return id;
}

/** Opens the work times of a person as the administrator, from the person's page. */
export async function openWorkTimesOf(page: Page, person: Person, id: string): Promise<void> {
  await page.goto(paths.people);
  await page.goto(personWorkTimesAddress(id));
  await expect(
    page.getByRole("heading", {
      level: 1,
      name: t("work-times.title-other", { name: person.name }),
    }),
  ).toBeVisible();
}

/** The window of an away time. */
export function awayWindow(page: Page): Locator {
  return page.getByRole("dialog");
}

/** Opens *Add away time* on the open page of work times. */
export async function openAddAwayTime(page: Page): Promise<Locator> {
  await page.getByRole("button", { name: t("work-times.away-add"), exact: true }).click();
  const window = awayWindow(page);
  await expect(window).toBeVisible();
  return window;
}

/** Fills the days (and the note, where the window has one) and adds the away time. */
export async function addAwayTime(
  page: Page,
  days: { readonly from: string; readonly to: string; readonly note?: string },
): Promise<void> {
  const window = await openAddAwayTime(page);
  await window.getByLabel(t("work-times.from"), { exact: true }).fill(days.from);
  await window.getByLabel(t("work-times.to"), { exact: true }).fill(days.to);
  if (days.note !== undefined) {
    await window.getByLabel(t("work-times.note"), { exact: true }).fill(days.note);
  }
  await window.getByRole("button", { name: t("work-times.save-add"), exact: true }).click();
  await expect(window).toBeHidden();
}

/** The list of the coming away times on the open page. */
export function comingAway(page: Page): Locator {
  return page.getByRole("region", { name: t("work-times.away-coming"), exact: true });
}

/** The person's own work times of today, through the API. */
export async function ownWorkTimes(
  server: Instance,
  person: Person,
): Promise<Schema<"WorkTimesBody">> {
  const session = await signInThroughApi(server, person);
  const today = dayFromToday(0);
  const read = dataOf(
    await session.client.GET("/api/v1/me/work-times", {
      params: { query: { from: today, to: today } },
    }),
  );
  await session.dispose();
  return read;
}

/** Sets the share of a person in a project through the API, as somebody who may. */
export async function setShareOf(
  server: Instance,
  who: Person,
  project: string,
  person: string,
  percent: number,
): Promise<void> {
  const session = await signInThroughApi(server, who);
  dataOf(
    await session.client.PUT("/api/v1/projects/{project}/shares/{person}", {
      params: { path: { project: await projectIdOf(session, project), person } },
      body: { percent },
    }),
  );
  await session.dispose();
}

/** The kinds of the notifications of a person. */
export async function notificationKindsOf(server: Instance, person: Person): Promise<string[]> {
  const session = await signInThroughApi(server, person);
  const list: Schema<"Page_NotificationEntry"> = dataOf(
    await session.client.GET("/api/v1/notifications"),
  );
  await session.dispose();
  return list.items.map((item) => item.kind);
}
