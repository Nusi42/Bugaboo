import { dataOf } from "@bugaboo/api-client";

import { ada, createPerson, type Person } from "./accounts.ts";
import { giveRole, roleName } from "./access.ts";
import { type ApiSession } from "./api.ts";
import { type Instance } from "./server.ts";

/** A person who holds *See working hours of others* and *Change working hours of others*: the role *HR*. */
export const hana: Person = {
  name: "Hana Hart",
  email: "hana@example.test",
  password: ada.password,
};

/** A team member who holds no permission of working hours. */
export const ben: Person = {
  name: "Ben Baker",
  email: "ben@example.test",
  password: ada.password,
};

/** Makes an account for `person` and gives them the role *HR*. */
export async function createHr(
  server: Instance,
  admin: ApiSession,
  person: Person = hana,
): Promise<void> {
  const session = await createPerson(server, admin, person);
  const me = dataOf(await session.client.GET("/api/v1/auth/me"));
  await giveRole(admin, me.id, roleName.hr, null);
  await session.dispose();
}
