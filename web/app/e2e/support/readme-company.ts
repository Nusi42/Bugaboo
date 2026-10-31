import { addToGroup, createGroup, giveGroupRole, giveRole, roleId, roleName } from "./access.ts";
import { ada, createAdmin, createPerson, ownId, type Person } from "./accounts.ts";
import { type ApiSession } from "./api.ts";
import { countIssueIds } from "./issues.ts";
import { createProject } from "./projects.ts";
import {
  company,
  groups,
  members,
  ownRoles,
  projects,
  roles,
  type Who,
} from "./readme-organization-data.ts";
import { inOrder } from "./sequence.ts";
import { type Instance } from "./server.ts";

// The people and the projects of the made-up workshop of the front page: Marta sets the company
// up, invites the others and gives them their roles, through the API.

/** A person of the workshop with an account: every one has the same made-up password. */
export function accountOf(who: Who): Person {
  return { ...members[who], password: ada.password };
}

/** Whether a word is the key of a person of the workshop. */
function isWho(key: string): key is Who {
  return key in members;
}

/** The people of the workshop: the administrator's session, the others' sessions and everybody's ID. */
export interface People {
  readonly admin: ApiSession;
  readonly sessions: Readonly<Partial<Record<Who, ApiSession>>>;
  readonly ids: Readonly<Partial<Record<Who, string>>>;
}

/** The ID of a person of the workshop. */
export function idOf(people: People, who: Who): string {
  const id = people.ids[who];
  if (id === undefined) {
    throw new TypeError(`${members[who].name} has no account.`);
  }
  return id;
}

/** A role by the key of a starting role (`teamMember`) or by the name of an own role. */
function roleCalled(key: string): string {
  return new Map(Object.entries(roleName)).get(key) ?? key;
}

/** The company, its projects and its own roles, as Marta sets them up first. */
async function setUpProjects(admin: ApiSession): Promise<void> {
  await countIssueIds(admin);
  const top = await createProject(admin, company.shortName, company.name);
  await inOrder(
    Object.values(projects).map(
      (project) => () => createProject(admin, project.shortName, project.name, top),
    ),
  );
  await inOrder(
    ownRoles.map((role) => async () => {
      await admin.client.POST("/api/v1/roles/{role}/copy", {
        params: { path: { role: await roleId(admin, roleName[role.copyOf]) } },
        body: { name: role.name },
      });
    }),
  );
}

/** Puts the people into their groups and gives the groups and some people their roles. */
async function giveRoles(people: People): Promise<void> {
  const { admin } = people;
  await inOrder(
    groups.map((group) => async () => {
      const id = await createGroup(admin, group.name);
      await inOrder(group.people.map((who) => () => addToGroup(admin, id, idOf(people, who))));
      await giveGroupRole(admin, id, roleCalled(group.role), company.shortName);
    }),
  );
  await inOrder(
    roles.map(
      ({ who, role, place }) =>
        () =>
          giveRole(
            admin,
            idOf(people, who),
            roleCalled(role),
            place === "company" ? company.shortName : projects[place].shortName,
          ),
    ),
  );
}

/** Sets up the company on an empty server: Marta, its projects and roles, and its people. */
export async function setUpCompany(server: Instance): Promise<People> {
  const admin = await createAdmin(server, accountOf("marta"));
  await setUpProjects(admin);
  const sessions: Partial<Record<Who, ApiSession>> = {};
  const ids: Partial<Record<Who, string>> = { marta: await ownId(admin) };
  await inOrder(
    Object.keys(members)
      .filter((key) => isWho(key))
      .filter((who) => who !== "marta")
      .map((who) => async () => {
        const session = await createPerson(server, admin, accountOf(who));
        sessions[who] = session;
        ids[who] = await ownId(session);
      }),
  );
  const people = { admin, sessions, ids };
  await giveRoles(people);
  return people;
}
