import { dataOf, settingKeys } from "@bugaboo/api-client";

import { giveRole, roleId, roleName, setFieldLevel, setPeopleLevel } from "./access.ts";
import { ownId, signInThroughApi } from "./accounts.ts";
import { theTwo } from "./demo-m3-state.ts";
import { BUG, CUSTOMER, customerField, guest, made, PROJECT } from "./demo-m4-state.ts";
import { fieldOf } from "./fields.ts";
import { firstIssueType, readIssue } from "./issues.ts";
import { startMailbox, turnMailOn } from "./mailbox.ts";
import { sampleAdministrator } from "./sample.ts";
import { type Instance } from "./server.ts";

// What the exit demo of M4 starts from (`m4-exit-demo.spec.ts`): the sample organization, with a mail
// server that takes the e-mail, and a Guest who reported a bug in Operations and wants to hear of
// it by e-mail. The workflow of Operations asks for who does the work (the sample made it so); the
// Guest cannot see the customer, nor who is assigned.

/**
 * Gets the sample ready for the demo. Everything here is what an administrator and the Guest do
 * through the API before the demo starts; the demo is about what happens next.
 */
export async function prepareTheDemo(server: Instance): Promise<void> {
  made.mailbox = await startMailbox();
  const admin = await signInThroughApi(server, sampleAdministrator());
  await turnMailOn(admin, made.mailbox);
  const guestSession = await signInThroughApi(server, guest);
  // The Guest of the Website has a role in Operations too, and cannot see who works on what there,
  // nor for which customer.
  const guestRole = await roleId(admin, roleName.guest);
  await giveRole(admin, await ownId(guestSession), roleName.guest, PROJECT);
  await setFieldLevel(
    admin,
    { role: guestRole, field: (await fieldOf(admin, customerField)).id },
    "hidden",
  );
  await setPeopleLevel(admin, { role: guestRole, kind: "assigned-to" }, "hidden");
  // The Guest reports the bug and asks to hear of what happens to it by e-mail too.
  const reported = dataOf(
    await guestSession.client.POST("/api/v1/projects/{project}/issues", {
      params: { path: { project: PROJECT } },
      body: { issue_type: await firstIssueType(guestSession), title: BUG },
    }),
  );
  if (reported.key === undefined || reported.key === null) {
    throw new TypeError("The server made an issue without a readable ID.");
  }
  made.key = reported.key;
  await guestSession.client.PUT("/api/v1/me/settings/{key}", {
    params: { path: { key: settingKeys.notificationsFollowedActivityByMail } },
    body: { value: true },
  });
  await guestSession.dispose();
  await admin.dispose();
}

/**
 * The Team member writes the customer of the bug, a field the Guest may not see (through the API:
 * the screens for it are tested with the issue page).
 */
export async function writeTheCustomer(server: Instance): Promise<void> {
  const session = await signInThroughApi(server, theTwo().first);
  const found = await readIssue(session, { key: made.key });
  const field = (await fieldOf(session, customerField)).id;
  dataOf(
    await session.client.PATCH("/api/v1/issues/{issue}", {
      params: { path: { issue: made.key } },
      body: { version: found.issue.version ?? 1, fields: { [field]: CUSTOMER } },
    }),
  );
  await session.dispose();
}
