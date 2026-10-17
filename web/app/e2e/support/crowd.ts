import { t } from "@bugaboo/messages";

import { createPeopleWithRoles, roleName } from "./access.ts";
import { ada, createAdmin, type Person } from "./accounts.ts";
import { createOwnField } from "./fields.ts";
import { countIssueIds, createIssue } from "./issues.ts";
import { createProject } from "./projects.ts";
import { type Instance } from "./server.ts";
import { inOrder } from "./sequence.ts";
import { serverWord } from "./server-words.ts";

// Ten people at the same time (architecture §11, M3 WP-10): the people, the project they work in,
// and a browser of its own for each of them.

/** How many people work at the same time. */
export const crowdSize = 10;

/** One of the nine Team members after the administrator. */
function member(first: string): Person {
  return {
    name: `${first} Crowd`,
    email: `${first.toLowerCase()}@example.test`,
    password: ada.password,
  };
}

export const ben = member("Ben");
export const cleo = member("Cleo");
export const dan = member("Dan");
export const eve = member("Eve");
export const finn = member("Finn");
export const gus = member("Gus");
export const hana = member("Hana");
export const ivo = member("Ivo");
export const jun = member("Jun");

/** The administrator (first) and nine Team members of the project WEB. */
export const crowd: readonly Person[] = [ada, ben, cleo, dan, eve, finn, gus, hana, ivo, jun];

/** The name of the own field *Area 3*. */
function areaName(number: number): string {
  return `Area ${String(number)}`;
}

/** The two choices of every area. */
export const areaChoices = ["Red", "Green"] as const;

/** The own fields that ten people change at once, one each (the status and the priority are two more). */
export const areaNames: readonly string[] = Array.from({ length: 8 }, (_, at) => areaName(at + 1));

/** What one person of the crowd changes on the issue they all have open: a field and its new value. */
export interface Edit {
  readonly person: Person;
  readonly label: string;
  readonly choice: string;
}

/** Ten people, ten fields of one issue: the status, the priority and the eight areas. */
export const crowdEdits: readonly Edit[] = [
  {
    person: ada,
    label: t("issue-field.status"),
    choice: serverWord("workflow-change.start-working"),
  },
  { person: ben, label: t("issue-field.priority"), choice: t("priority.critical") },
  ...[cleo, dan, eve, finn, gus, hana, ivo, jun].map((person, at) => ({
    person,
    label: areaName(at + 1),
    choice: areaChoices[1],
  })),
];

/** The issues the scenarios work on, by the use they have; `WEB-1` is the first one made. */
export const crowdIssues = {
  description: "WEB-1",
  status: "WEB-2",
  fields: "WEB-3",
  network: "WEB-4",
  manyChanges: ["WEB-5", "WEB-6", "WEB-7", "WEB-8"],
  manyChangeEditing: ["WEB-9", "WEB-10", "WEB-11", "WEB-12"],
} as const;

/** The titles of the issues, in the order they are made. */
const titles = [
  "Description edited by three",
  "Status changed twice",
  "Ten fields at once",
  "A network that drops",
  "Many issues, first",
  "Many issues, second",
  "Many issues, third",
  "Many issues, fourth",
  "Many issues under edit, first",
  "Many issues under edit, second",
  "Many issues under edit, third",
  "Many issues under edit, fourth",
];

/**
 * Gets a server ready for ten people: the project WEB with twelve issues (WEB-1 to WEB-12), eight
 * own fields of two choices, and nine Team members next to the administrator.
 */
export async function prepareCrowd(server: Instance): Promise<void> {
  const admin = await createAdmin(server);
  await countIssueIds(admin);
  await createProject(admin, "WEB", "Website");
  await inOrder(
    areaNames.map((name) => () => createOwnField(admin, name, "pick-one", areaChoices)),
  );
  await inOrder(
    titles.map((title) => async () => {
      await createIssue(admin, "WEB", title, `The text of: ${title}`);
    }),
  );
  await createPeopleWithRoles(
    server,
    admin,
    crowd.slice(1).map((person) => ({ person, role: roleName.teamMember, project: "WEB" })),
  );
  await admin.dispose();
}
