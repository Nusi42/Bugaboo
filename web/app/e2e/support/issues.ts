import { dataOf, settingKeys, type SettingChoice } from "@bugaboo/api-client";

import { ana, lea, makePeopleWithRoles, roleName } from "./access.ts";
import { bob, makeAdmin, signInThroughApi } from "./accounts.ts";
import { type ApiSession } from "./api.ts";
import { makeProject } from "./projects.ts";
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

/** Makes an issue in a project (given by its short name) as the person of the session. */
export async function makeIssue(
  session: ApiSession,
  project: string,
  title: string,
  description?: string,
): Promise<MadeIssue> {
  const made = dataOf(
    await session.client.POST("/api/v1/projects/{project}/issues", {
      params: { path: { project } },
      body: {
        issue_type: await firstIssueType(session),
        title,
        ...(description === undefined ? {} : { description }),
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
        what: { record: { kind: "issue", id: issue.id }, part: "description" },
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
  const admin = await makeAdmin(server);
  // Counting issue IDs, so the tests can name them: WEB-1, WEB-2, WEB-3.
  await admin.client.PUT("/api/v1/settings/{key}", {
    params: { path: { key: settingKeys.issuesIdMode } },
    body: { value: "counting" },
  });
  await makeProject(admin, "WEB", "Website");
  await makeIssue(admin, "WEB", webIssues.login, "Steps:\n\n1. Reset the password");
  await makeIssue(admin, "WEB", webIssues.passkey);
  await makePeopleWithRoles(server, admin, [
    { person: ana, role: roleName.viewer, project: "WEB" },
    { person: bob, role: roleName.teamMember, project: "WEB" },
    { person: lea, role: roleName.guest, project: "WEB" },
  ]);
  const guest = await signInThroughApi(server, lea);
  await makeIssue(guest, "WEB", webIssues.typo);
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
      what: { record: { kind: "issue", id: issue.id }, part: "description" },
      editor_id: crypto.randomUUID(),
    },
  });
}

/** An issue that exists already, found by its readable ID. */
export async function issueOf(session: ApiSession, key: string): Promise<MadeIssue> {
  const found = dataOf(
    await session.client.GET("/api/v1/issues/{issue}", { params: { path: { issue: key } } }),
  );
  const id = found.issue.id;
  if (id === undefined || id === null) {
    throw new TypeError(`The issue ${key} has no ID.`);
  }
  return { id, key };
}

/** The version an issue has now. */
export async function versionOf(session: ApiSession, issue: MadeIssue): Promise<number> {
  const found = dataOf(
    await session.client.GET("/api/v1/issues/{issue}", { params: { path: { issue: issue.key } } }),
  );
  return found.issue.version ?? 1;
}

/** Makes a project and one issue in it; the issue. */
export async function makeProjectWithIssue(
  admin: ApiSession,
  shortName: string,
  name: string,
): Promise<MadeIssue> {
  await makeProject(admin, shortName, name);
  return makeIssue(admin, shortName, webIssues.login);
}
