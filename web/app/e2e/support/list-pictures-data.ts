import { signedInAdmin } from "./access.ts";
import { bob, signInThroughApi, ownId } from "./accounts.ts";
import { assignTo, issueOf, prepareManyWeb, versionOf } from "./issues.ts";
import { createLabel } from "./labels.ts";
import { prepareLook } from "./list-look.ts";
import { type Instance } from "./server.ts";
import { inOrder } from "./sequence.ts";

/**
 * The list of the look with what the rows of a long list show: people assigned and labels on the
 * issues, so the pictures can show a row with all its cells.
 */
export async function prepareFull(server: Instance): Promise<void> {
  await prepareLook(server);
  const admin = await signedInAdmin(server);
  const teamMember = await signInThroughApi(server, bob);
  const [login, passkey] = [await issueOf(admin, "WEB-1"), await issueOf(admin, "WEB-2")];
  await assignTo(admin, login, await ownId(admin), await versionOf(admin, login));
  await assignTo(admin, passkey, await ownId(teamMember), await versionOf(admin, passkey));
  await inOrder(
    (
      [
        ["login", login],
        ["passkey", passkey],
      ] as const
    ).map(([name, issue]) => async () => {
      const label = await createLabel(admin, name);
      await admin.client.POST("/api/v1/issues/{issue}/labels", {
        params: { path: { issue: issue.key } },
        body: { label },
      });
    }),
  );
  await teamMember.dispose();
  await admin.dispose();
}

/** A project with more issues than a page of the list: the offer to tick every result. */
export async function prepareLong(server: Instance): Promise<void> {
  await prepareManyWeb(server, 130);
}
