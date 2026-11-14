import { signedInAdmin } from "./access.ts";
import { ISSUE_TITLE } from "./change-row.ts";
import { OWN } from "./changes-issue-more.ts";
import { LABEL } from "./changes-issue-page.ts";
import { createOwnField } from "./fields.ts";
import { createIssue } from "./issues.ts";
import { createLabel } from "./labels.ts";
import { inOrder } from "./sequence.ts";
import type { Instance } from "./server.ts";
import { prepareTimeline } from "./timeline.ts";

/**
 * Gets a server ready for the table: the project WEB with the scene of the timeline, the issue the
 * changes of the issue page are made on, the label of
 * the change of labels, and the own fields of the changes of own fields.
 */
export async function prepareChanges(server: Instance): Promise<void> {
  await prepareTimeline(server);
  const admin = await signedInAdmin(server);
  await createIssue(admin, "WEB", ISSUE_TITLE);
  await createLabel(admin, LABEL);
  await inOrder([
    () => createOwnField(admin, OWN.one.name, "pick-one", OWN.one.choices),
    () => createOwnField(admin, OWN.several.name, "pick-several", OWN.several.choices),
    () => createOwnField(admin, OWN.text.name, "text"),
  ]);
  await admin.dispose();
}
