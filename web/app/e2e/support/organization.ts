import { dataOf, type SettingKey } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { type Browser, expect } from "@playwright/test";

import { agentsEnabledKey, aiAppsEnabledKey } from "../../src/shared/ai-kinds.ts";
import { paths } from "../../src/shared/paths.ts";
import { ada, addCodeApp, createPerson, ownId, type Person, signInThroughApi } from "./accounts.ts";
import { giveRole, roleName, signedInAdmin } from "./access.ts";
import { type ApiSession } from "./api.ts";
import { type Instance } from "./server.ts";

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
  const made = await createPerson(server, admin, keeper);
  await giveRole(admin, await ownId(made), roleName.administrator, null);
  const { app } = await addCodeApp(made, keeper.password);
  await made.dispose();
  await admin.dispose();

  const session = await signInThroughApi(server, keeper);
  await session.client.POST("/api/v1/auth/second-step", { body: { code: await app.next() } });
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
  key: SettingKey,
  on: boolean,
): Promise<void> {
  const keepers = await keeperOf(server);
  const path = { params: { path: { key } } };
  await (on
    ? keepers.client.PUT("/api/v1/settings/{key}", { ...path, body: { value: true } })
    : keepers.client.DELETE("/api/v1/settings/{key}", path));
}

/**
 * Switches AI apps and agents on for the organization, as an administrator does on their pages:
 * both start off on a new installation (COMP-04), and the tests of agents and AI apps need them.
 * `admin` is the session of an administrator.
 */
export async function turnAiAppsAndAgentsOn(admin: ApiSession): Promise<void> {
  const turnOn = async (key: string): Promise<void> => {
    dataOf(
      await admin.client.PUT("/api/v1/settings/{key}", {
        params: { path: { key } },
        body: { value: true },
      }),
    );
  };
  // One after the other: each change is a new version of the organization's settings.
  await turnOn(agentsEnabledKey);
  await turnOn(aiAppsEnabledKey);
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
