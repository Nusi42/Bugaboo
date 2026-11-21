import { dataOf } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { expect, type Locator, type Page } from "@playwright/test";

import { personAddress } from "../../src/shared/paths.ts";
import { lea, signedInAdmin } from "./access.ts";
import { signInThroughApi, type Person } from "./accounts.ts";
import { type ApiSession } from "./api.ts";
import { prepareWebWithAgents } from "./agents-scene.ts";
import { type Instance } from "./server.ts";
import { createDays, daysAgo } from "./working-hours-scene.ts";

/** The job writes the file of a person's data in the background: seconds on a quiet computer. */
export const FILE_CREATED_MS = 60_000;

/** A day long past every keep period of time entries: 2014, more than ten years before the tests run. */
export const longAgo = "2014-03-03";

/** Books half an hour on WEB-1 on `date`, as the person of the session; the ID of the entry. */
export async function bookOn(session: ApiSession, date: string): Promise<string> {
  const made = dataOf(
    await session.client.POST("/api/v1/me/time-entries", {
      body: { issue: "WEB-1", date, minutes: 30 },
    }),
  );
  return made.id;
}

/**
 * The server of the privacy screens: the one of the agents (WEB with Ana, Bob and Lea, AI apps and agents on), and a
 * work day of Lea three days ago, which the law keeps after her removal.
 */
export async function preparePrivacy(server: Instance): Promise<void> {
  await prepareWebWithAgents(server);
  const session = await signInThroughApi(server, lea);
  await createDays(session, [daysAgo(3)]);
  await session.dispose();
}

/** An administrator at the other end of the API, with a time entry of 2014 and one of today on WEB-1. */
export async function adminWithOldAndNewTime(server: Instance): Promise<ApiSession> {
  const admin = await signedInAdmin(server);
  await bookOn(admin, longAgo);
  await bookOn(admin, daysAgo(0));
  return admin;
}

/**
 * Opens the page of a person as the administrator: their ID from the list of people, which also finds a person who is
 * switched off and cannot sign in to say it themselves.
 */
export async function openPersonPage(page: Page, server: Instance, person: Person): Promise<void> {
  const admin = await signedInAdmin(server);
  const found = dataOf(
    await admin.client.GET("/api/v1/people", { params: { query: { search: person.email } } }),
  ).items[0]?.person;
  await admin.dispose();
  if (found?.shown !== "name") {
    throw new TypeError(`${person.email} is not in the list of people.`);
  }
  await page.goto(personAddress(found.id));
  await expect(page.getByRole("heading", { level: 1, name: person.name })).toBeVisible();
}

/** The card *Sign-in and removal* of a person's page. */
export function stateCard(page: Page): Locator {
  return page.getByRole("region", { name: t("person.state-title"), exact: true });
}

/** The window of *Remove a person* for the person called `name`. */
export function removalWindow(page: Page, name: string): Locator {
  return page.getByRole("dialog", { name: t("removal.title", { name }) });
}
