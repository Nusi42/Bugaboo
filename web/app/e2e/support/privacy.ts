import { dataOf } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { expect, type Locator, type Page, type Route } from "@playwright/test";

import { personAddress } from "../../src/shared/paths.ts";
import { lea, signedInAdmin } from "./access.ts";
import { ada, signInThroughApi, type Person } from "./accounts.ts";
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

/** The ID of a person, from the list of people as an administrator reads it: a switched-off person is found too. */
export async function idInPeople(admin: ApiSession, person: Person): Promise<string> {
  const found = dataOf(
    await admin.client.GET("/api/v1/people", { params: { query: { search: person.email } } }),
  ).items[0]?.person;
  if (found?.shown !== "name") {
    throw new TypeError(`${person.email} is not in the list of people.`);
  }
  return found.id;
}

/**
 * Opens the page of a person as the administrator: their ID from the list of people, which also finds a person who is
 * switched off and cannot sign in to say it themselves.
 */
export async function openPersonPage(page: Page, server: Instance, person: Person): Promise<void> {
  const admin = await signedInAdmin(server);
  const id = await idInPeople(admin, person);
  await admin.dispose();
  await openPersonById(page, id, person.name);
}

/** Opens the page of the person with this ID and waits for their name as its heading. */
export async function openPersonById(page: Page, id: string, name: string): Promise<void> {
  await page.goto(personAddress(id));
  await expect(page.getByRole("heading", { level: 1, name })).toBeVisible();
}

/** Switches a person off, as an administrator does on their page. */
export async function switchOffByApi(admin: ApiSession, person: string): Promise<void> {
  await admin.client.POST("/api/v1/people/{person}/switch-off", { params: { path: { person } } });
}

/**
 * Removes a switched-off person, with what the organization's setting offers for their comments,
 * as the administrator of the first start (`signedInAdmin`), whose password a removal asks for again.
 */
export async function removeByApi(admin: ApiSession, person: string): Promise<void> {
  await admin.client.POST("/api/v1/people/{person}/remove", {
    params: { path: { person } },
    body: { password: ada.password },
  });
}

/** The card *Sign-in and removal* of a person's page. */
export function stateCard(page: Page): Locator {
  return page.getByRole("region", { name: t("person.state-title"), exact: true });
}

/** The window of *Remove a person* for the person called `name`. */
export function removalWindow(page: Page, name: string): Locator {
  return page.getByRole("dialog", { name: t("removal.title", { name }) });
}

/**
 * Answers the page of a person as the server does, but with the removal still going on: the job of a removal is done in
 * a moment on the small organization of a test, so a picture of *removing…* holds the answer.
 */
export async function stillRemoving(route: Route): Promise<void> {
  const found = await route.fetch();
  const body: unknown = await found.json();
  const person = typeof body === "object" && body !== null ? body : {};
  await route.fulfill({ response: found, json: { ...person, removing: true } });
}
