import { t } from "@bugaboo/messages";
import { expect, type Locator, type Page } from "@playwright/test";

import { ada, makePerson, type Person } from "./accounts.ts";
import { type ApiSession, openSession } from "./api.ts";
import { type Instance } from "./server.ts";
import { layoutOf } from "./layout.ts";
import { inOrder } from "./sequence.ts";

export const ana: Person = {
  name: "Ana Analyst",
  email: "ana@example.test",
  password: ada.password,
};

export const lea: Person = {
  name: "Lea Lead",
  email: "lea@example.test",
  password: ada.password,
};

/** The roles of the starting set, by the name people read. */
export const roleName = {
  administrator: t("starting-role.administrator"),
  projectLead: t("starting-role.project-lead"),
  teamMember: t("starting-role.team-member"),
  contributor: t("starting-role.contributor"),
  guest: t("starting-role.guest"),
} as const;

/** Whether something the server sent is a role of the table: a name and an ID. */
function isRole(value: unknown): value is { readonly id: string; readonly name: string } {
  return (
    typeof value === "object" &&
    value !== null &&
    typeof Reflect.get(value, "id") === "string" &&
    typeof Reflect.get(value, "name") === "string"
  );
}

/** The ID of a role of the organization, found by its name, as an administrator lists them. */
export async function roleId(admin: ApiSession, name: string): Promise<string> {
  const table = await admin.get("/api/v1/field-permissions");
  const roles: unknown = table["roles"];
  const found = Array.isArray(roles)
    ? roles.filter((role) => isRole(role)).find((role) => role.name === name)
    : undefined;
  if (found === undefined) {
    throw new TypeError(`The organization has no role called ${name}.`);
  }
  return found.id;
}

/** Takes away what a project set for a field and a role, so the level from above counts again. */
export async function forgetFieldLevel(
  admin: ApiSession,
  project: string,
  role: string,
  field: string,
): Promise<void> {
  await admin.put(`/api/v1/field-permissions/${role}/${field}?project=${project}`, {
    level: null,
  });
}

/**
 * The list of levels of one role for one field or kind, wherever the screen keeps it: in the cell
 * of the matrix on a computer, and in the list of the chosen role on a phone and on a tablet.
 */
export async function levelList(page: Page, subject: string, role: string): Promise<Locator> {
  if (layoutOf(page) === "computer") {
    return page.getByLabel(t("access.cell-label", { subject, role }), { exact: true });
  }
  await page.getByLabel(t("access.role-chooser"), { exact: true }).selectOption({ label: role });
  return page.getByLabel(subject, { exact: true });
}

/** Picks a level in the list and waits until the server has it (*Saved*). */
export async function pickLevel(list: Locator, page: Page, level: string): Promise<void> {
  await list.selectOption({ label: level });
  await expect(page.getByText(t("save.saved"), { exact: true })).toBeVisible();
}

/** The cell or the item around a list of levels: where its source is told (*From ACME*) and *Reset* is. */
export function around(list: Locator): Locator {
  return list.locator("xpath=ancestor::*[self::td or self::li][1]");
}

/** An administrator at the other end of the API, signed in with the password of the first start. */
export async function signedInAdmin(server: Instance): Promise<ApiSession> {
  const session = await openSession(server.url);
  await session.post("/api/v1/auth/sign-in", { email: ada.email, password: ada.password });
  return session;
}

/** The ID of the person behind a session. */
export async function ownId(session: ApiSession): Promise<string> {
  return String((await session.get("/api/v1/auth/me"))["id"]);
}

/**
 * Gives a person a role the way *Who has which role* does: for the whole organization, or on a
 * project (given by its short name) and its sub-projects (M2-F92).
 */
export async function giveRole(
  admin: ApiSession,
  person: string,
  role: string,
  project: string | null,
): Promise<void> {
  const row = { who: { person }, role: await roleId(admin, role) };
  await (project === null
    ? admin.post("/api/v1/assignments", row)
    : admin.post(`/api/v1/projects/${await projectIdOf(admin, project)}/assignments`, {
        ...row,
        applies_to: "project-and-sub-projects",
      }));
}

/** A person who is made, and the role they are given on a project and its sub-projects. */
export interface WithRole {
  readonly person: Person;
  readonly role: string;
  readonly project: string;
}

/** Makes an account for each person and gives each the role they are named with (M2-F92). */
export async function makePeopleWithRoles(
  server: Instance,
  admin: ApiSession,
  who: readonly WithRole[],
): Promise<void> {
  await inOrder(
    who.map(({ person, role, project }) => async () => {
      const session = await makePerson(server, admin, person);
      await giveRole(admin, await ownId(session), role, project);
      await session.dispose();
    }),
  );
}

/** The ID of a project, found by its address (a short name). */
export async function projectIdOf(admin: ApiSession, shortName: string): Promise<string> {
  const found = await admin.get(`/api/v1/projects/${shortName}`);
  const project: unknown = found["project"];
  const id: unknown =
    typeof project === "object" && project !== null ? Reflect.get(project, "id") : undefined;
  if (typeof id !== "string") {
    throw new TypeError(`There is no project ${shortName}.`);
  }
  return id;
}
