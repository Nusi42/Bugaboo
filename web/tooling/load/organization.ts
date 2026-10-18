import {
  fieldsHiddenFrom,
  giveRole,
  roleId,
  roleName,
  setFieldLevel,
} from "../../app/e2e/support/access.ts";
import {
  ada,
  createAdmin,
  createPerson,
  ownId,
  type Person,
} from "../../app/e2e/support/accounts.ts";
import { type ApiSession } from "../../app/e2e/support/api.ts";
import { createOwnField } from "../../app/e2e/support/fields.ts";
import { countIssueIds, createIssue } from "../../app/e2e/support/issues.ts";
import { createProject } from "../../app/e2e/support/projects.ts";
import { inOrder } from "../../app/e2e/support/sequence.ts";
import { type Instance } from "../../app/e2e/support/server.ts";
import { askForWhoWorks } from "../../app/e2e/support/workflows.ts";
import { type Work } from "./book.ts";

/** How many issues the people share: few, so that they meet. */
const ISSUES = 6;

/** What the own field that the Team members write and Guests may not see is called. */
export const HIDDEN_FIELD_NAME = "Internal note";

/** The people of a run: the administrator first, then Team members, and the Guest last. */
function peopleOf(count: number): readonly Person[] {
  return [
    ada,
    ...Array.from({ length: count - 1 }, (_, at) => ({
      name: `Load Person ${String(at + 2)}`,
      email: `load${String(at + 2)}@example.test`,
      password: ada.password,
    })),
  ];
}

/** What a run starts from. */
export interface Prepared {
  /** The sessions of the people, the administrator first and the Guest last. */
  readonly sessions: readonly ApiSession[];
  readonly ids: readonly string[];
  /** The issues the Team members share, and last the one the Guest reported. */
  readonly issues: readonly Work[];
  /** The issue the Guest reported. */
  readonly guestsIssue: Work;
  /** The key of the own field that Guests may not see. */
  readonly hiddenField: string;
  /** Every field Guests may not see: the built-in ones their role hides and the own field. */
  readonly hiddenFromGuests: readonly string[];
}

/**
 * The organization of a run: one project with a workflow of its own that asks for who does the
 * work, an own field Guests may not see, the issues, and the people with their sessions: the
 * administrator, Team members, and a Guest who reports an issue of their own.
 */
export async function prepare(server: Instance, count: number): Promise<Prepared> {
  const admin = await createAdmin(server);
  await countIssueIds(admin);
  await createProject(admin, "LOAD", "Load");
  await askForWhoWorks(admin, "LOAD");
  const field = await createOwnField(admin, HIDDEN_FIELD_NAME, "text");
  const guestRole = await roleId(admin, roleName.guest);
  await setFieldLevel(admin, { role: guestRole, field: field.id }, "hidden");
  const shared: Work[] = [];
  await inOrder(
    Array.from({ length: ISSUES }, (_, at) => async () => {
      shared.push(await createIssue(admin, "LOAD", `Shared issue ${String(at + 1)}`));
    }),
  );
  const people = peopleOf(count);
  const guestPerson = people.at(-1);
  if (guestPerson === undefined) {
    throw new TypeError("A run needs a Guest.");
  }
  const sessions: ApiSession[] = [admin];
  await inOrder(
    people.slice(1, -1).map((person) => async () => {
      const session = await createPerson(server, admin, person);
      await giveRole(admin, await ownId(session), roleName.teamMember, "LOAD");
      sessions.push(session);
    }),
  );
  const guest = await createPerson(server, admin, guestPerson);
  const guestId = await ownId(guest);
  await giveRole(admin, guestId, roleName.guest, "LOAD");
  const guestsIssue: Work = {
    ...(await createIssue(guest, "LOAD", "An issue the Guest reported")),
    reporter: guestId,
  };
  sessions.push(guest);
  return {
    sessions,
    ids: await Promise.all(sessions.map((session) => ownId(session))),
    issues: [...shared, guestsIssue],
    guestsIssue,
    hiddenField: field.id,
    hiddenFromGuests: await fieldsHiddenFrom(admin, guestRole),
  };
}
