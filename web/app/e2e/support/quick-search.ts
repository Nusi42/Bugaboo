import { dataOf } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { expect, type Locator, type Page } from "@playwright/test";

import { doNothing } from "../../src/shared/do-nothing.ts";
import { createPeopleWithRoles, giveRole, roleName } from "./access.ts";
import { bob, createAdmin, ownId, signInThroughApi } from "./accounts.ts";
import { type ApiSession } from "./api.ts";
import { countIssueIds, createIssue, setStatusAndPriority, versionOf } from "./issues.ts";
import { createProject } from "./projects.ts";
import { type Instance } from "./server.ts";

// The scenes of the tests of quick search (CFG-08, M5-Q3): an organization with a project that
// Bob sees and one that only the administrator sees, each with issues, and a saved search of Ada's.

/** The titles of the issues of the scene. */
export const titles = {
  login: "Login fails after password reset",
  passkey: "Sign in with a passkey",
  secret: "Secret login plans",
  mail: "E-mail bounces",
  random: "Random kind issue",
  colourQueued: "Colour check queued",
  colourRunning: "Colour check running",
  colourDone: "Colour check finished",
  colourLow: "Colour check later",
} as const;

/** The status and the priority the login issue and the passkey issue get: the groups In progress and Done, Critical and High. */
const loginLook = { status: "in-progress", priority: "critical" } as const;
const passkeyLook = { status: "done", priority: "high" } as const;

/** The name of the saved search of the administrator, which nobody else may open. */
export const savedSearchName = "Login bugs";

/** The time between two keys of a fast typist: well under the pause the app waits for. */
export const FAST_TYPING_MS = 30;

/** Whether the search by words finds a title for the administrator: the index of titles is made from the outbox. */
async function titleIsFound(admin: ApiSession, word: string): Promise<boolean> {
  const found = dataOf(
    await admin.client.GET("/api/v1/search/issues", {
      params: { query: { q: `title contains "${word}"`, limit: 1 } },
    }),
  );
  return found.items.length > 0;
}

/** Makes the scene, and waits until the index of titles holds the issues. */
export async function prepareScene(server: Instance): Promise<void> {
  const admin = await createAdmin(server);
  // The issue of this project is made before the organization is set to counting IDs, so its
  // key is one with random letters (`RND-K7Q2X`) and keeps it.
  await createProject(admin, "RND", "Random kinds");
  await createIssue(admin, "RND", titles.random);
  await countIssueIds(admin);
  await createProject(admin, "WEB", "Website");
  await createProject(admin, "SECRET", "Secret plans");
  const login = await createIssue(admin, "WEB", titles.login);
  const passkey = await createIssue(admin, "WEB", titles.passkey);
  await createIssue(admin, "WEB", titles.mail);
  // One issue of each status group and of different priorities, so the rows of the results differ as the rows of the list.
  await setStatusAndPriority(admin, login, await versionOf(admin, login), loginLook);
  await setStatusAndPriority(admin, passkey, await versionOf(admin, passkey), passkeyLook);
  await createIssue(admin, "WEB", titles.colourQueued);
  const running = await createIssue(admin, "WEB", titles.colourRunning);
  const finished = await createIssue(admin, "WEB", titles.colourDone);
  const later = await createIssue(admin, "WEB", titles.colourLow);
  await setStatusAndPriority(admin, running, await versionOf(admin, running), {
    status: "in-progress",
    priority: "high",
  });
  await setStatusAndPriority(admin, finished, await versionOf(admin, finished), {
    status: "done",
    priority: "critical",
  });
  // It stays Not started: only its priority changes.
  await admin.client.PATCH("/api/v1/issues/{issue}", {
    params: { path: { issue: later.key } },
    body: { version: await versionOf(admin, later), priority: "low" },
  });
  await createIssue(admin, "SECRET", titles.secret);
  await createPeopleWithRoles(server, admin, [
    { person: bob, role: roleName.teamMember, project: "WEB" },
  ]);
  // The same person in a second project.
  const bobSession = await signInThroughApi(server, bob);
  await giveRole(admin, await ownId(bobSession), roleName.teamMember, "RND");
  await bobSession.dispose();
  await admin.client.POST("/api/v1/saved-searches", {
    body: { name: savedSearchName, search_text: 'priority is "high"' },
  });
  await expect.poll(() => titleIsFound(admin, "secret")).toBe(true);
  await expect.poll(() => titleIsFound(admin, "passkey")).toBe(true);
  await expect.poll(() => titleIsFound(admin, "colour")).toBe(true);
  await expect.poll(() => titleIsFound(admin, "random")).toBe(true);
  await admin.dispose();
}

/** The dialog of quick search. */
export function dialog(page: Page) {
  return page.getByRole("dialog", { name: t("search.title"), exact: true });
}

/** The box of quick search. */
export function box(page: Page) {
  return dialog(page).getByRole("searchbox", { name: t("search.label"), exact: true });
}

/** Opens quick search and types into its box. */
export async function ask(page: Page, text: string): Promise<void> {
  await page.keyboard.press("Control+k");
  await expect(dialog(page)).toBeVisible();
  await box(page).fill(text);
}

/** The list of results. */
export function list(page: Page) {
  return dialog(page).getByRole("listbox", { name: t("search.results") });
}

/** The empty state under the box: its text is said once more by the announcement, so it is found by its part. */
export function nothingFound(page: Page) {
  return dialog(page).locator("[data-part='quick-search.empty']");
}

/**
 * What a result that is an issue shows as the row of the list does (R4, R9), found by parts and names, not by colors:
 * the tint of its status group and the edge of its priority are states of the row; the priority is an icon named by its word.
 */
export function rowLook(option: Locator) {
  return {
    option,
    statusIcon: option.locator("[data-part='status-icon']"),
    priority: option.locator("[data-part='priority']"),
    typeIcon: option.locator("[data-part='icon'][role='img']"),
  };
}

/** The results of the list that say this text. */
export function result(page: Page, text: string) {
  return list(page).getByRole("option").filter({ hasText: text });
}

/** The status with which the server answers a request for an issue, as the signed-in person. */
export async function statusOfIssue(page: Page, key: string): Promise<number> {
  const answer = await page.request.get(new URL(`/api/v1/issues/${key}`, page.url()).href);
  return answer.status();
}

/**
 * Holds back the answers to the searches of issues until the returned function is called, so a test
 * can look at the dialog while the server has yet to answer. The app gives up on a search that is
 * held back longer than `sourceTimeoutMs`, so a test that holds it must release it sooner.
 */
export async function holdBackSearches(page: Page): Promise<() => void> {
  let release = doNothing;
  const gate = new Promise<void>((resolve) => {
    release = resolve;
  });
  await page.route("**/api/v1/search/issues*", async (route) => {
    await gate;
    await route.continue();
  });
  return release;
}

/** The box names the result that has the focus of the list: it is the first option, with its id. */
export async function expectBoxPointsAtFirst(page: Page): Promise<void> {
  await expect.poll(() => pointsAtFirst(page)).toBe(true);
}

/** Whether the box points at the option that is first in the list, now. */
export async function pointsAtFirst(page: Page): Promise<boolean> {
  const first = list(page).getByRole("option").first();
  const pointed = await box(page).getAttribute("aria-activedescendant");
  return pointed !== null && pointed === (await first.getAttribute("id"));
}

/** The title of the result that has the focus of the list. */
export function focusedTitle(page: Page) {
  return list(page).locator("[data-focused='true'] [data-part='quick-search.title']");
}
