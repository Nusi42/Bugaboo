import { dataOf } from "@bugaboo/api-client";
import { t, textByKey } from "@bugaboo/messages";
import { expect, type Page } from "@playwright/test";

import { projectIssuesAddress, savedSearchAddress } from "../../src/shared/paths.ts";
import { signInThroughApi } from "./accounts.ts";
import { goodInFourThemes } from "./demo.ts";
import { addressText, box, closeBar, openBar, openSearch } from "./search-bar.ts";
import { sampleNamed, sampleTeamMember, type SamplePerson } from "./sample.ts";
import { sampleInGroup } from "./sample-groups.ts";
import { type Instance } from "./server.ts";

// The steps of the exit demo of M5 (`m5-exit-demo.spec.ts`) on the sample organization: searching in
// both modes and switching between them, a field that a Contributor may not read, the searches that
// people of the sample saved and shared, the list a project shows, and Quick search.

/** The pictures of the demo, in the folder of the screenshots. */
const DEMO_M5 = "M5/demo";

/** The search of the first step: what a Team member looks at first. */
export const HIGH_PRIORITY = "priority is high";
/** The search that reads the own field *Customer*, which Contributors may not read. */
export const CUSTOMER_SEARCH = 'customer is "Alder Street Bakery"';
/** The saved search that reads *Customer*, shared with a group and with the project WEB. */
const CUSTOMER_SEARCH_NAME = "Questions of customers";
/** The saved search the project WEB shows as its list. */
export const PROJECT_LIST_NAME = "High priority, not done";

/** The screen is good in the four themes and is in the pictures of this demo. */
export function good(page: Page, step: string): Promise<void> {
  return goodInFourThemes(page, step, DEMO_M5);
}

/** Ana Moreno, a Contributor in Website through the group External, who may not read *Customer*. */
export function contributor(): SamplePerson {
  return sampleNamed("Ana Moreno");
}

/** Step 1: Simple search of the sample; the list has issues and says how many. */
export async function simpleSearchFindsIssues(page: Page): Promise<void> {
  await openSearch(page, HIGH_PRIORITY, sampleTeamMember());
  await expect(page.getByText(/issues? found$/u).first()).toBeVisible();
  await good(page, "01-simple-search");
}

/** Step 2: Advanced search shows the same text, and back to Simple loses nothing. */
export async function switchingKeepsTheSearch(page: Page): Promise<void> {
  await openSearch(page, `${HIGH_PRIORITY} and statusGroup is not Done`, sampleTeamMember());
  await openBar(page);
  const text = addressText(page);
  await page.getByRole("button", { name: t("search.bar.to-advanced") }).click();
  await expect(box(page)).toHaveValue(text);
  await good(page, "02-advanced-search");
  await page.getByRole("button", { name: t("search.bar.to-simple") }).click();
  await page.getByRole("button", { name: t("search.bar.to-advanced") }).click();
  await expect(box(page)).toHaveValue(text);
  expect(addressText(page)).toBe(text);
}

/** Step 3: a Contributor who may not read *Customer* gets the answer of a field that is not there. */
export async function aHiddenFieldIsAnUnknownField(page: Page, server: Instance): Promise<void> {
  // The Contributor looks at a list and then types the search: the box says what is wrong while the
  // list of the last good search stays behind it, with its note.
  await openSearch(page, HIGH_PRIORITY, contributor());
  await openBar(page);
  await page.getByRole("button", { name: t("search.bar.to-advanced") }).click();
  await box(page).fill(CUSTOMER_SEARCH);
  const hidden = await mistakeIn(page);
  expect(hidden).toBe(textByKey("search.mistake.unknown-field", { field: "customer" }));
  await box(page).press("Escape");
  await expect(page.getByText(t("search.found-stale"))).toBeVisible();
  await page.waitForLoadState("networkidle");
  await good(page, "03-a-field-that-is-not-there");
  // The words are those for a field that does not exist, with only the typed word changed.
  await box(page).fill('customex is "Alder Street Bakery"');
  expect((await mistakeIn(page)).replace("customex", "customer")).toBe(hidden);
  // The Team member who may read it is told nothing is wrong: the search is answered.
  const team = await signInThroughApi(server, sampleTeamMember());
  const answer = await team.client.GET("/api/v1/search/issues", {
    params: { query: { q: CUSTOMER_SEARCH, limit: 1 } },
  });
  await team.dispose();
  expect(answer.response.status).toBe(200);
}

/** The words of the first mistake under the box of Advanced search. */
async function mistakeIn(page: Page): Promise<string> {
  const mistake = page.locator("[data-part='search-mistake']").first();
  await expect(mistake).toBeVisible();
  return (await mistake.innerText()).trim();
}

/** The saved searches shared with the person (by the route a page asks), by name. */
async function sharedWith(server: Instance, person: Readonly<SamplePerson>) {
  const session = await signInThroughApi(server, person);
  const shared = dataOf(
    await session.client.GET("/api/v1/saved-searches", { params: { query: { scope: "shared" } } }),
  );
  await session.dispose();
  return shared.items;
}

/** Step 4: the shared searches reach people who read them, and the one with *Customer* no Contributor. */
export async function sharedSearchesReachWhoReadThem(page: Page, server: Instance): Promise<void> {
  const team = sampleInGroup("Frontend");
  let found: { readonly id: string; readonly name: string } | undefined;
  for (const each of await sharedWith(server, team)) {
    if (each.name === CUSTOMER_SEARCH_NAME) {
      found = { id: each.id, name: each.name };
    }
  }
  expect(found, "the Team member is offered the search that reads Customer").toBeDefined();
  const names: string[] = [];
  for (const each of await sharedWith(server, contributor())) {
    names.push(each.name);
  }
  expect(names).not.toContain(CUSTOMER_SEARCH_NAME);
  // Shared with the same project, the search that does not read it does reach the Contributor.
  expect(names).toContain(PROJECT_LIST_NAME);
  await openSearch(page, "", team);
  await page.goto(savedSearchAddress(found?.id ?? ""));
  await expect(page.getByRole("heading", { level: 1, name: found?.name ?? "" })).toBeVisible();
  // The customer is one of the sample, so the search finds issues.
  await expect(page.getByText(/issues? found$/u).first()).toBeVisible();
  await good(page, "04-a-shared-search");
}

/** Step 5: the project shows the saved search chosen as its list. */
export async function theProjectShowsItsList(page: Page): Promise<void> {
  await openSearch(page, "", sampleTeamMember());
  await page.goto(projectIssuesAddress("WEB"));
  // The list of the project starts from the chosen search: its filter stands in the bar.
  await openBar(page);
  await expect(
    page.getByRole("button", { name: t("search.bar.remove-filter", { filter: "Priority: high" }) }),
  ).toBeVisible();
  await closeBar(page);
  await good(page, "05-the-list-of-a-project");
}
