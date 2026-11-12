import { dataOf } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { expect, type Locator, type Page } from "@playwright/test";

import { issueAddress } from "../../src/shared/paths.ts";
import { ada, bob, ownId, signInThroughApi, type Person } from "./accounts.ts";
import { lea } from "./access.ts";
import { detailRow, issueOf, prepareWeb, versionOf } from "./issues.ts";
import { showDetails } from "./labels.ts";
import { inOrder } from "./sequence.ts";
import { type Instance } from "./server.ts";

// The scene of the tests of the time factor (PLAN-14, ADR-0030): the project WEB of the tests of
// issues, with WEB-1 (made by Ada) and WEB-3 (reported by Lea, a Guest) both assigned to Bob, a Team
// member, and estimated at 8 hours.

/** The estimate of the issues of the scene, in minutes. */
export const ESTIMATE_MINUTES = 480;

/** The issues of the scene: one everybody in WEB reads, and one the Guest Lea reads too. */
export const factorIssues = { ada: "WEB-1", lea: "WEB-3" } as const;

/** Gets the server ready: WEB, and its two issues assigned to Bob with an estimate. */
export async function prepareTimeFactor(server: Instance): Promise<void> {
  await prepareWeb(server);
  const admin = await signInThroughApi(server, ada);
  const bobId = await idOfPerson(server, bob);
  await inOrder(
    Object.values(factorIssues).map((key) => async () => {
      const issue = await issueOf(admin, key);
      dataOf(
        await admin.client.PATCH("/api/v1/issues/{issue}", {
          params: { path: { issue: key } },
          body: {
            version: await versionOf(admin, issue),
            assigned_to: [bobId],
            estimated_time: ESTIMATE_MINUTES,
          },
        }),
      );
    }),
  );
  await admin.dispose();
}

/** The ID of a person, read by signing them in through the API. */
export async function idOfPerson(server: Instance, person: Person): Promise<string> {
  const session = await signInThroughApi(server, person);
  const id = await ownId(session);
  await session.dispose();
  return id;
}

/** The time factor of an issue as a person reads it through the API. */
export async function factorAs(server: Instance, person: Person, key: string) {
  const session = await signInThroughApi(server, person);
  const read = dataOf(
    await session.client.GET("/api/v1/issues/{issue}/time-factor", {
      params: { path: { issue: key } },
    }),
  );
  await session.dispose();
  return read;
}

/** Sets the time factor of an issue through the API, as a person who may. */
export async function setFactorAs(
  server: Instance,
  person: Person,
  key: string,
  percent: number,
): Promise<void> {
  const version = (await factorAs(server, person, key)).version;
  const session = await signInThroughApi(server, person);
  dataOf(
    await session.client.PUT("/api/v1/issues/{issue}/time-factor", {
      params: { path: { issue: key } },
      body: { version, percent },
    }),
  );
  await session.dispose();
}

/** Opens an issue page with all its Details (a phone and a tablet fold some of them away). */
export async function openIssue(page: Page, key: string): Promise<void> {
  await page.goto(issueAddress(key));
  await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
  await showDetails(page);
  await expect(page.getByText(t("issue-field.assigned-to"), { exact: true })).toBeVisible();
}

/**
 * A sentence of the history of the open issue about the time factor, with the value `value` (a
 * pattern of its words): the name of the field and the value stand in one sentence there, never in
 * the row of the Details, where they are apart.
 */
export function historyLine(page: Page, value: string): Locator {
  return page.getByText(new RegExp(`${t("issue-field.time-factor")}.*${value}`, "u"));
}

/** The row *Time factor* of the Details. */
export function factorRow(page: Page): Locator {
  return detailRow(page, t("issue-field.time-factor"));
}

export { lea };
