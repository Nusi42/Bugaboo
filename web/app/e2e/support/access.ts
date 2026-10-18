import { dataOf, type Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { expect, type Locator, type Page } from "@playwright/test";

import { ada, createPerson, ownId, type Person, signInThroughApi } from "./accounts.ts";
import { type ApiSession } from "./api.ts";
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
  viewer: t("starting-role.viewer"),
  contributor: t("starting-role.contributor"),
  guest: t("starting-role.guest"),
} as const;

/** The IDs of the roles of the organization by name, as an administrator lists them. */
export async function roleIds(admin: ApiSession): Promise<ReadonlyMap<string, string>> {
  const table = dataOf(await admin.client.GET("/api/v1/field-permissions"));
  return new Map(table.roles.map((role) => [role.name, role.id]));
}

/** The ID of a role of the organization, found by its name, as an administrator lists them. */
export async function roleId(admin: ApiSession, name: string): Promise<string> {
  const found = (await roleIds(admin)).get(name);
  if (found === undefined) {
    throw new TypeError(`The organization has no role called ${name}.`);
  }
  return found;
}

/**
 * The fields a role cannot see, as the table of field permissions says them (one source with the
 * server): a built-in field by its word, an own field by its ID.
 */
export async function fieldsHiddenFrom(
  admin: ApiSession,
  role: string,
): Promise<readonly string[]> {
  const table = dataOf(await admin.client.GET("/api/v1/field-permissions"));
  const hidden: string[] = [];
  for (const row of table.rows) {
    for (const cell of row.cells) {
      if (cell.role === role && cell.level === "hidden") {
        hidden.push(row.subject);
        break;
      }
    }
  }
  return hidden;
}

/** Makes a group through the API and returns its ID. */
export async function createGroup(admin: ApiSession, name: string): Promise<string> {
  return dataOf(await admin.client.POST("/api/v1/groups", { body: { name } })).id;
}

/** Puts a person into a group, through the API. */
export async function addToGroup(admin: ApiSession, group: string, person: string): Promise<void> {
  await admin.client.PUT("/api/v1/groups/{group_id}/members/{person_id}", {
    params: { path: { group_id: group, person_id: person } },
  });
}

/** Sets the level of a role for a field, in the whole organization or in a project (an ID). */
export async function setFieldLevel(
  admin: ApiSession,
  where: { readonly role: string; readonly field: Schema<"FieldKey">; readonly project?: string },
  level: Schema<"FieldLevel"> | null,
): Promise<void> {
  await admin.client.PUT("/api/v1/field-permissions/{role}/{field}", {
    params: {
      path: { role: where.role, field: where.field },
      ...(where.project === undefined ? {} : { query: { project: where.project } }),
    },
    body: { level },
  });
}

/** Sets how a role sees people of one kind, in the whole organization. */
export async function setPeopleLevel(
  admin: ApiSession,
  where: { readonly role: string; readonly kind: Schema<"PeopleKind"> },
  level: Schema<"PeopleLevel">,
): Promise<void> {
  await admin.client.PUT("/api/v1/people-visibility/{role}/{kind}", {
    params: { path: { role: where.role, kind: where.kind } },
    body: { level },
  });
}

/** Takes away what a project set for a field and a role, so the level from above counts again. */
export function forgetFieldLevel(
  admin: ApiSession,
  project: string,
  role: string,
  field: Schema<"FieldKey">,
): Promise<void> {
  return setFieldLevel(admin, { role, field, project }, null);
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
export function signedInAdmin(server: Instance): Promise<ApiSession> {
  return signInThroughApi(server, ada);
}

/** Whom a row of *Who has which role* is for: a person or a group (by ID). */
type Who = { readonly person: string } | { readonly group: string };

/**
 * Gives a role, by its ID, the way *Who has which role* does: for the whole organization, or on a
 * project (given by its short name), by default with its sub-projects (M2-F92).
 */
async function give(
  admin: ApiSession,
  who: Who,
  role: string,
  project: string | null,
  appliesTo: Schema<"AppliesToBody">,
): Promise<void> {
  await (project === null
    ? admin.client.POST("/api/v1/assignments", { body: { who, role } })
    : admin.client.POST("/api/v1/projects/{project}/assignments", {
        params: { path: { project: await projectIdOf(admin, project) } },
        body: { who, role, applies_to: appliesTo },
      }));
}

/** Gives a person a role, by its name: for the whole organization, or on a project and its sub-projects. */
export async function giveRole(
  admin: ApiSession,
  person: string,
  role: string,
  project: string | null,
  appliesTo: Schema<"AppliesToBody"> = "project-and-sub-projects",
): Promise<void> {
  await give(admin, { person }, await roleId(admin, role), project, appliesTo);
}

/** Gives a person a role that is known by its ID (a role made a moment ago, among very many). */
export function giveRoleById(admin: ApiSession, person: string, role: string): Promise<void> {
  return give(admin, { person }, role, null, "project-and-sub-projects");
}

/** Gives a group a role, by its name: for the whole organization, or on a project and its sub-projects. */
export async function giveGroupRole(
  admin: ApiSession,
  group: string,
  role: string,
  project: string | null,
): Promise<void> {
  await give(admin, { group }, await roleId(admin, role), project, "project-and-sub-projects");
}

/** A person who is made, and the role they are given on a project and its sub-projects. */
export interface WithRole {
  readonly person: Person;
  readonly role: string;
  readonly project: string;
}

/** Makes an account for each person and gives each the role they are named with (M2-F92). */
export async function createPeopleWithRoles(
  server: Instance,
  admin: ApiSession,
  who: readonly WithRole[],
): Promise<void> {
  await inOrder(
    who.map(({ person, role, project }) => async () => {
      const session = await createPerson(server, admin, person);
      await giveRole(admin, await ownId(session), role, project);
      await session.dispose();
    }),
  );
}

/** The ID of a project, found by its address (a short name). */
export async function projectIdOf(admin: ApiSession, shortName: string): Promise<string> {
  const found = dataOf(
    await admin.client.GET("/api/v1/projects/{project}", {
      params: { path: { project: shortName } },
    }),
  );
  return found.project.id;
}

/**
 * Chooses whom to check, and where when `place` is named, on the page of *Check access*: the
 * search finds them and the radio button picks them.
 */
export async function check(
  page: Page,
  place: string | null,
  person: { readonly name: string },
): Promise<void> {
  if (place !== null) {
    await page.getByLabel(t("check-access.place-search")).fill(place);
    await page.getByRole("radio", { name: new RegExp(place, "u") }).check();
  }
  await page.getByLabel(t("check-access.person-search")).fill(person.name);
  await page.getByRole("radio", { name: person.name }).check();
}
