import { dataOf } from "@bugaboo/api-client";

import { roleId, roleName, setFieldLevel, setPeopleLevel } from "./access.ts";
import { ownId, signInThroughApi } from "./accounts.ts";
import { assignedTo, COST_CENTER, costCenter, made, theTwo } from "./demo-m3-state.ts";
import { addToTheFormOfWeb } from "./layouts.ts";
import { sampleAdministrator } from "./sample.ts";
import { type ApiSession } from "./api.ts";
import { type Instance } from "./server.ts";

// What the exit demo of M3 starts from (`m3-exit-demo.spec.ts`).

/**
 * What the demo starts from: a project lead gave the form of the Website a few more fields for
 * everybody but Guests, and the administrator hid from Guests who is assigned (the sample's
 * Website keeps the starting rules, so there is something to change).
 */
export async function prepareTheDemo(server: Instance): Promise<void> {
  const admin = await signInThroughApi(server, sampleAdministrator());
  // Cost center is a field of time entries until M6, so the Website asks for one of its own.
  const field = dataOf(
    await admin.client.POST("/api/v1/fields", {
      body: { name: costCenter, kind: "text", applies_to: "issues" },
    }),
  );
  const guest = await roleId(admin, roleName.guest);
  await setFieldLevel(admin, { role: guest, field: field.id }, "hidden");
  await addToTheFormOfWeb(admin, [assignedTo, costCenter]);
  await setPeopleLevel(
    admin,
    { role: await roleId(admin, roleName.guest), kind: "assigned-to" },
    "hidden",
  );
  await admin.dispose();
}

/** The ID of the own field *Cost center* of the Website. */
export async function costCenterField(session: ApiSession): Promise<string> {
  for (const each of dataOf(await session.client.GET("/api/v1/fields")).fields) {
    if (each.name === costCenter) {
      return each.id;
    }
  }
  throw new Error("The Website has no field called Cost center.");
}

/**
 * The Team member takes the Guest's bug: assigns it to a colleague and writes the cost center of
 * the work (through the API; the screens for it are tested with the issue page).
 */
export async function takeTheGuestsBug(server: Instance): Promise<void> {
  const { first, second } = theTwo();
  const session = await signInThroughApi(server, first);
  const other = await signInThroughApi(server, second);
  const colleague = await ownId(other);
  await other.dispose();
  const found = dataOf(
    await session.client.GET("/api/v1/issues/{issue}", {
      params: { path: { issue: made.guestKey } },
    }),
  );
  const field = await costCenterField(session);
  await session.client.PATCH("/api/v1/issues/{issue}", {
    params: { path: { issue: made.guestKey } },
    body: {
      version: found.issue.version ?? 1,
      assigned_to: [colleague],
      fields: { [field]: COST_CENTER },
    },
  });
  await session.dispose();
}
