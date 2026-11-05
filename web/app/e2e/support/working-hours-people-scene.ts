import { dataOf } from "@bugaboo/api-client";

import { createPeopleWithRoles, giveRole, roleName, signedInAdmin } from "./access.ts";
import { type ApiSession } from "./api.ts";
import { bob, cleo, createPerson, ownId } from "./accounts.ts";
import { type Instance } from "./server.ts";
import { prepareWorkingHours } from "./working-hours-scene.ts";

/** The role that may change the settings of the organization and nothing else. */
const SETTINGS_ONLY = "Settings only";

/** Makes the role *Settings only* and gives it to a person (by ID) for the whole organization. */
export async function giveSettingsOnlyRole(admin: ApiSession, person: string): Promise<void> {
  dataOf(
    await admin.client.POST("/api/v1/roles", {
      body: {
        name: SETTINGS_ONLY,
        permissions: ["change-organization-settings"],
        two_step: false,
      },
    }),
  );
  await giveRole(admin, person, SETTINGS_ONLY, null);
}

/**
 * The scene of *Working hours* with two more people for the pictures: Bob holds a role with *Change
 * organization settings* only (the card of who sees working hours says why it has no list), and Cleo
 * leads the sub-project APP only (the project above, with the default, is a project she cannot see).
 */
export async function prepareWorkingHoursPeople(server: Instance): Promise<void> {
  await prepareWorkingHours(server);
  const admin = await signedInAdmin(server);
  const colleague = await createPerson(server, admin, bob);
  await giveSettingsOnlyRole(admin, await ownId(colleague));
  await colleague.dispose();
  await createPeopleWithRoles(server, admin, [
    { person: cleo, role: roleName.projectLead, project: "APP" },
  ]);
  await admin.dispose();
}
