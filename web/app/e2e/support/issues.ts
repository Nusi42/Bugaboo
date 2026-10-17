import {
  dataOf,
  descriptionEditPart,
  firstVersion,
  settingKeys,
  type Schema,
  type SettingChoice,
} from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { expect, type Page } from "@playwright/test";

import { ana, lea, createPeopleWithRoles, roleName } from "./access.ts";
import { bob, createAdmin, signInThroughApi } from "./accounts.ts";
import { type ApiSession } from "./api.ts";
import { createProject } from "./projects.ts";
import { type Instance } from "./server.ts";

/** An issue that was made through the API. */
export interface MadeIssue {
  /** The internal ID. */
  readonly id: string;
  /** The readable ID, such as `WEB-1`. */
  readonly key: string;
}

/** The ID of the first issue type of the first start (*Bug*). */
export async function firstIssueType(session: ApiSession): Promise<string> {
  const types = dataOf(await session.client.GET("/api/v1/issue-types"));
  const first = types.items[0]?.id;
  if (first === undefined || first === null) {
    throw new TypeError("The server has no issue type.");
  }
  return first;
}

/**
 * The ID of the issue type the scenarios make issues of: *Task*, which uses the starter workflow
 * *Simple*, where any status can follow any other (a *Bug* has to pass *Testing*).
 */
export async function taskIssueType(session: ApiSession): Promise<string> {
  const types = dataOf(await session.client.GET("/api/v1/issue-types"));
  const task = types.items.find((each) => each.name === "Task")?.id;
  if (task === undefined || task === null) {
    throw new TypeError("The server has no issue type Task.");
  }
  return task;
}

/** Makes an issue in a project (given by its short name) as the person of the session. */
export async function createIssue(
  session: ApiSession,
  project: string,
  title: string,
  description?: string,
  fields?: Readonly<Record<string, unknown>>,
): Promise<MadeIssue> {
  const made = dataOf(
    await session.client.POST("/api/v1/projects/{project}/issues", {
      params: { path: { project } },
      body: {
        issue_type: await taskIssueType(session),
        title,
        ...(description === undefined ? {} : { description }),
        ...(fields === undefined ? {} : { fields }),
      },
    }),
  );
  if (made.id === undefined || made.id === null || made.key === undefined || made.key === null) {
    throw new TypeError("The server made an issue without an ID.");
  }
  return { id: made.id, key: made.key };
}

/** Chooses where the person opens issues: in the focus view or in the side panel. */
export async function openIssuesIn(
  session: ApiSession,
  where: SettingChoice<"appearanceIssueView">,
): Promise<void> {
  await session.client.PUT("/api/v1/me/settings/{key}", {
    params: { path: { key: settingKeys.appearanceIssueView } },
    body: { value: where },
  });
}

/** Takes the lock on the description of an issue and saves a text, as another person's tab would. */
export async function saveDescription(
  session: ApiSession,
  issue: MadeIssue,
  text: string,
  version: number,
): Promise<void> {
  const taken = dataOf(
    await session.client.POST("/api/v1/edit-locks", {
      body: {
        what: { record: { kind: "issue", id: issue.id }, part: descriptionEditPart },
        editor_id: crypto.randomUUID(),
      },
    }),
  );
  if (taken.status !== "taken") {
    throw new TypeError("Somebody else is editing.");
  }
  await session.client.PUT("/api/v1/issues/{issue}/description", {
    params: { path: { issue: issue.key } },
    body: { text, version, lock_token: taken.lock_token },
  });
}

/** The issues of the project WEB that the tests of issues start from, by their titles. */
export const webIssues = {
  login: "Login fails after password reset",
  passkey: "Sign in with a passkey",
  typo: "Typo on the contact page",
} as const;

/**
 * Gets a server ready for the tests of issues: the project WEB with two issues the administrator
 * made, a Viewer (Ana) and a Guest (Lea) with a role in it, and an issue the Guest reported.
 */
export async function prepareWeb(server: Instance): Promise<void> {
  const admin = await createAdmin(server);
  // Counting issue IDs, so the tests can name them: WEB-1, WEB-2, WEB-3.
  await countIssueIds(admin);
  await createProject(admin, "WEB", "Website");
  await createIssue(admin, "WEB", webIssues.login, "Steps:\n\n1. Reset the password");
  await createIssue(admin, "WEB", webIssues.passkey);
  await createPeopleWithRoles(server, admin, [
    { person: ana, role: roleName.viewer, project: "WEB" },
    { person: bob, role: roleName.teamMember, project: "WEB" },
    { person: lea, role: roleName.guest, project: "WEB" },
  ]);
  const guest = await signInThroughApi(server, lea);
  await createIssue(guest, "WEB", webIssues.typo);
  await guest.dispose();
  await admin.dispose();
}

/** Assigns an issue to a person (by ID), on the version the test knows. */
export async function assignTo(
  session: ApiSession,
  issue: MadeIssue,
  person: string,
  version: number,
): Promise<void> {
  await session.client.PATCH("/api/v1/issues/{issue}", {
    params: { path: { issue: issue.key } },
    body: { version, assigned_to: [person] },
  });
}

/** Takes the lock on the description of an issue and keeps it (for a minute): somebody is editing. */
export async function holdDescription(session: ApiSession, issue: MadeIssue): Promise<void> {
  await session.client.POST("/api/v1/edit-locks", {
    body: {
      what: { record: { kind: "issue", id: issue.id }, part: descriptionEditPart },
      editor_id: crypto.randomUUID(),
    },
  });
}

/** The issue as the session reads it. */
export async function readIssue(session: ApiSession, issue: Pick<MadeIssue, "key">) {
  return dataOf(
    await session.client.GET("/api/v1/issues/{issue}", { params: { path: { issue: issue.key } } }),
  );
}

/** Makes the organization count its issue IDs (WEB-1, WEB-2, …) instead of making them up. */
export async function countIssueIds(admin: ApiSession): Promise<void> {
  await admin.client.PUT("/api/v1/settings/{key}", {
    params: { path: { key: settingKeys.issuesIdMode } },
    body: { value: "counting" },
  });
}

/** An issue that exists already, found by its readable ID. */
export async function issueOf(session: ApiSession, key: string): Promise<MadeIssue> {
  const found = await readIssue(session, { key });
  const id = found.issue.id;
  if (id === undefined || id === null) {
    throw new TypeError(`The issue ${key} has no ID.`);
  }
  return { id, key };
}

/** The version an issue has now. */
export async function versionOf(session: ApiSession, issue: MadeIssue): Promise<number> {
  return (await readIssue(session, issue)).issue.version ?? firstVersion;
}

/** Makes a project and one issue in it; the issue. */
export async function createProjectWithIssue(
  admin: ApiSession,
  shortName: string,
  name: string,
): Promise<MadeIssue> {
  await createProject(admin, shortName, name);
  return createIssue(admin, shortName, webIssues.login);
}

/** The *Saved* note of a page. */
export function savedNote(page: Page) {
  return page.getByText(new RegExp(`^${t("save.saved")}`, "u")).first();
}

/**
 * The *Saved* note under the title of the issue page: the one for a change that has no row in the
 * Details (the title, the status). A change in a row says *Saved* in its own row, not here.
 */
export function issueNote(page: Page) {
  return page.locator('[data-part="issue-note"]').getByText(new RegExp(`^${t("save.saved")}`, "u"));
}

/** The name of the box of the title on the form of a new issue: every required field says so. */
export function titleLabel(): string {
  return t("fields.label-required", { field: t("issue-field.title") });
}

/** The dialog that makes an issue. */
export function newIssueDialog(page: Page) {
  return page.getByRole("dialog", { name: t("issues.new-title"), exact: true });
}

/** Opens the dialog with the button of the bar: the sidebar on a computer, the bar of a phone. */
export async function openNewIssueDialog(page: Page): Promise<void> {
  await page
    .getByRole("button", { name: t("issues.new"), exact: true })
    .first()
    .click();
  await expect(newIssueDialog(page)).toBeVisible();
}

/** Opens the dialog with the button in the menu of a phone. */
export async function openNewIssueDialogFromTheMenu(page: Page): Promise<void> {
  await page
    .getByRole("navigation", { name: t("shell.menu") })
    .getByRole("button", { name: t("issues.new"), exact: true })
    .click();
  await expect(newIssueDialog(page)).toBeVisible();
}

/** Changes the status and the priority of an issue as the session's person, on the version the test knows. */
export async function setStatusAndPriority(
  session: ApiSession,
  issue: MadeIssue,
  version: number,
  change: { readonly status: string; readonly priority: Schema<"Priority"> },
): Promise<void> {
  await session.client.PATCH("/api/v1/issues/{issue}", {
    params: { path: { issue: issue.key } },
    body: { version, ...change },
  });
}

/** The tick box of an issue in the list, named as the list names it. */
export function tickBox(page: Page, key: string) {
  return page.getByRole("checkbox", { name: t("issues.select-issue", { key }), exact: true });
}

/** The big status button of the issue page: the current status with its group's icon and color. */
export function statusButton(page: Page) {
  return page.locator('[data-part="status-button"]');
}

/** Changes the status the way a person does: opens the list of the big button and picks a word. */
export async function chooseStatus(page: Page, word: string): Promise<void> {
  await statusButton(page).click();
  await page.getByRole("button", { name: word, exact: true }).click();
}

/** The big status button says this status. */
export async function expectStatus(page: Page, word: string): Promise<void> {
  await expect(statusButton(page)).toHaveText(word);
}

/** The row of a field in the Details of the issue page, found by the name of the field. */
export function detailRow(page: Page, name: string) {
  return page
    .locator('[data-part="name-value"]')
    .filter({ has: page.getByText(name, { exact: true }) });
}

/** Changes a field of the issue page the way a person does: the status with the big button, any other with its list. */
export async function changeField(page: Page, name: string, choice: string): Promise<void> {
  await (name === t("issue-field.status")
    ? chooseStatus(page, choice)
    : page.getByLabel(name, { exact: true }).selectOption({ label: choice }));
}
