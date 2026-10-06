import { t } from "@bugaboo/messages";
import { expect, type Locator, type Page } from "@playwright/test";

import { ada, type Person } from "./accounts.ts";
import { type ApiSession, openSession } from "./api.ts";
import { type Instance } from "./server.ts";
import { layoutOf } from "./layout.ts";

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
