import { dataOf } from "@bugaboo/api-client";

import { roleId, roleName } from "./access.ts";
import { signInThroughApi } from "./accounts.ts";
import {
  sampleAdministrator,
  sampleGuestIn,
  sampleHolderIn,
  sampleNamed,
  sampleNames,
  type SamplePerson,
} from "./sample.ts";
import { type Instance } from "./server.ts";

/** The project with sub-projects that the tour of the projects is about, and one of its sub-projects. */
export const PROJECT = "APP";
export const SUB_PROJECT = "IOS";

/** An issue of the sample that the issue screens open, and its project: the Website counts its IDs. */
export const ISSUE = "WEB-12";
export const ISSUE_PROJECT = "WEB";

/** Who looks at a screen: nobody is signed in, or a person of the sample with a role. */
export type Who = "nobody" | "administrator" | "teamMember" | "contributor" | "guest";

/** What the showcase opens on pages of its own, by the names of the sample (`tools/dev/sample.json`). */
export const showcaseNames = {
  person: "Ben Hale",
  group: sampleNames.groups["backend"] ?? "",
  role: sampleNames.roles["security-reviewer"] ?? "",
} as const;

/** The person of the sample behind each role of the tour; `nobody` is not signed in. */
export const personOf: Readonly<Record<Exclude<Who, "nobody">, () => SamplePerson>> = {
  administrator: sampleAdministrator,
  teamMember: () => sampleNamed(showcaseNames.person),
  contributor: () => sampleHolderIn(roleName.contributor, PROJECT),
  guest: () => sampleGuestIn(PROJECT),
};

/** The IDs of the person, the group and the role the showcase opens on their own pages. */
export interface ShowcaseIds {
  readonly person: string;
  readonly group: string;
  readonly role: string;
}

/** Finds the person, the group and the role of `showcaseNames`, as an administrator lists them. */
export async function showcaseIds(server: Instance): Promise<ShowcaseIds> {
  const admin = await signInThroughApi(server, sampleAdministrator());
  const people = dataOf(
    await admin.client.GET("/api/v1/people", {
      params: { query: { search: showcaseNames.person } },
    }),
  );
  const groups = dataOf(
    await admin.client.GET("/api/v1/groups", { params: { query: { fields: "id,name" } } }),
  );
  const role = await roleId(admin, showcaseNames.role);
  await admin.dispose();
  let person: string | undefined;
  for (const item of people.items) {
    if (item.person?.shown === "name") {
      person = item.person.id;
      break;
    }
  }
  let group: string | undefined;
  for (const item of groups.items) {
    if (item.name === showcaseNames.group && typeof item.id === "string") {
      group = item.id;
    }
  }
  if (person === undefined || group === undefined) {
    throw new TypeError(
      `The sample has no ${showcaseNames.person} or no group ${showcaseNames.group}.`,
    );
  }
  return { person, group, role };
}
