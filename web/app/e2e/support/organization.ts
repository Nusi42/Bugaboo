import { t } from "@bugaboo/messages";
import { type Browser, expect } from "@playwright/test";

import { paths } from "../../src/shared/paths.ts";
import { ada, makePerson, type Person } from "./accounts.ts";
import { giveRole, ownId, roleName, signedInAdmin } from "./access.ts";
import { type ApiSession, openSession } from "./api.ts";
import { type Instance } from "./server.ts";
import { codeApp } from "./totp.ts";

/** The setting of the organization that asks everybody for a second step at sign-in. */
export const TWO_STEP_REQUIRED = "security.two-step-required";

/**
 * Somebody who keeps the switch: an Administrator with a code app, so that the organization
 * asking for a second step does not lock them out of the settings that turn it off again.
 */
const keeper: Person = {
  name: "Kai Keeper",
  email: "kai@example.test",
  password: ada.password,
};

/** The signed-in session of the keeper, kept for the life of the server: a code is used once. */
const sessions = new WeakMap<Instance, ApiSession>();

/** Makes the keeper (with the first administrator's help) and signs them in with password and code. */
async function keeperOf(server: Instance): Promise<ApiSession> {
  const known = sessions.get(server);
  if (known !== undefined) {
    return known;
  }
  const admin = await signedInAdmin(server);
  const made = await makePerson(server, admin, keeper);
  await giveRole(admin, await ownId(made), roleName.administrator, null);
  const started = await made.post("/api/v1/auth/two-step/code-app", { password: keeper.password });
  const app = codeApp(String(started["secret"]));
  await made.post("/api/v1/auth/two-step/code-app/confirm", {
    code: await app.next(),
    password: keeper.password,
  });
  await made.dispose();
  await admin.dispose();

  const session = await openSession(server.url);
  await session.post("/api/v1/auth/sign-in", { email: keeper.email, password: keeper.password });
  await session.post("/api/v1/auth/second-step", { code: await app.next() });
  sessions.set(server, session);
  return session;
}

/**
 * Switches a yes-or-no setting of the organization the way an administrator does on *Sign-in
 * rules*: a change of the organization's value, through the API (M2-F50). It counts from the next
 * request, as a change made on the screen would. Turning it off takes the value away, so the
 * standard counts again.
 */
export async function setOrganizationSwitch(
  server: Instance,
  key: string,
  on: boolean,
): Promise<void> {
  const keepers = await keeperOf(server);
  await (on
    ? keepers.put(`/api/v1/settings/${key}`, { value: true })
    : keepers.remove(`/api/v1/settings/${key}`));
}

/**
 * Presses the switch *Two-step sign-in for everybody* on *Sign-in rules*, as the keeper does in a
 * browser of their own, and waits until the server has the change (*Saved*) (M2-F50).
 */
export async function pressTwoStepSwitch(
  browser: Browser,
  server: Instance,
  on: boolean,
): Promise<void> {
  const session = await keeperOf(server);
  const context = await browser.newContext({
    baseURL: server.url,
    ignoreHTTPSErrors: true,
    storageState: await session.state(),
  });
  const page = await context.newPage();
  await page.goto(paths.signInRules);
  const name = t("setting.security.two-step-required.name");
  const field = page.getByRole("switch", { name, exact: true });
  await expect(field).toBeVisible();
  if ((await field.isChecked()) !== on) {
    await page.getByText(name, { exact: true }).click();
  }
  await (on ? expect(field).toBeChecked() : expect(field).not.toBeChecked());
  await expect(page.getByText(t("save.saved"), { exact: true })).toBeVisible();
  await context.close();
}
