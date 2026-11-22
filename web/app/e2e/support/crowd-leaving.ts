import { giveRole, roleName, signedInAdmin } from "./access.ts";
import { ada, createPerson, ownId, type Person } from "./accounts.ts";
import { ben, crowdEdits, crowdIssues, type Edit, prepareCrowd } from "./crowd.ts";
import { comment } from "./notifications.ts";
import { type Instance } from "./server.ts";

// Ten people at the same time while people leave and a backup is made (`concurrent/privacy.spec.ts`,
// PRIV-04, OPS-07): who leaves, and what each of the ten changes.

/** The Team member who leaves while the others work: they wrote a comment on the issue of the ten. */
export const lou: Person = {
  name: "Lou Leaving",
  email: "lou@example.test",
  password: ada.password,
};

/** The crowd of ten, and Lou, who wrote a comment on the issue the ten change. */
export async function prepareCrowdWithLeaver(server: Instance): Promise<void> {
  await prepareCrowd(server);
  const admin = await signedInAdmin(server);
  const session = await createPerson(server, admin, lou);
  await giveRole(admin, await ownId(session), roleName.teamMember, "WEB");
  await comment(session, crowdIssues.fields, "Handing this over before I leave.");
  await Promise.all([session.dispose(), admin.dispose()]);
}

/** What the ten change, and the issues they work on. */
export { crowdEdits, crowdIssues } from "./crowd.ts";

/** Ben, whose page is open when he is switched off. */
export { ben } from "./crowd.ts";

/** The changes of everybody but Ben, who is switched off. */
export const othersThanBen: readonly Edit[] = crowdEdits.filter((edit) => edit.person !== ben);

/** Ben's change: the one his open page sends after he was switched off. */
export const bensEdit: Edit = (() => {
  const found = crowdEdits.find((edit) => edit.person === ben);
  if (found === undefined) {
    throw new TypeError("Ben has no field to change.");
  }
  return found;
})();
