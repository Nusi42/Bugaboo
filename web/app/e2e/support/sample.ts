import { cpSync, existsSync, readFileSync } from "node:fs";
import { join, resolve } from "node:path";

import layout from "../../../../tools/dev/sample.json" with { type: "json" };
import { repoRoot } from "../../../tooling/ports.ts";
import { type Person } from "./accounts.ts";

/** Somebody of the sample organization: the person and how they are known in the sample. */
export interface SamplePerson extends Person {
  /** The role the person is given once roles exist, as the sample names it (`team_member`). */
  readonly role: string;
  /** The secret of the code app, for the people who have one. */
  readonly codeAppSecret: string | undefined;
  /** The access key, for the few people who have one. */
  readonly accessKey: string | undefined;
}

function sampleFolder(): string {
  return resolve(repoRoot, layout.out);
}

/** Fails with what to do when the sample was not made. */
function requireSample(): void {
  if (!existsSync(join(sampleFolder(), layout.signInFile))) {
    throw new Error(
      `Make the sample organization first: cargo xtask dataset sample (it makes ${sampleFolder()}).`,
    );
  }
}

/**
 * The people of the sample and their random passwords of this run, from the ignored file
 * `sign-in.txt`. One line per person: e-mail address, password, role, name and, for people with a
 * code app, its secret, and for the few with an access key, the key, separated by tabs (an empty
 * column stays empty); lines that start with `#` are comments.
 */
export function samplePeople(): readonly SamplePerson[] {
  requireSample();
  return readFileSync(join(sampleFolder(), layout.signInFile), "utf8")
    .split("\n")
    .map((line) => line.replace(/\r$/u, ""))
    .filter((line) => line.trim() !== "" && !line.startsWith("#"))
    .map((line) => {
      const [email, password, role, name, secret, key] = line.split("\t");
      if (
        email === undefined ||
        password === undefined ||
        role === undefined ||
        name === undefined
      ) {
        throw new Error(`A line of ${layout.signInFile} does not have its four columns.`);
      }
      return {
        name,
        email,
        password,
        role,
        codeAppSecret: secret === "" ? undefined : secret,
        accessKey: key === "" ? undefined : key,
      };
    });
}

/** A person of the sample that matches, or an error that says what the sample lacks. */
function somebody(what: string, matches: (person: SamplePerson) => boolean): SamplePerson {
  const found = samplePeople().find((person) => matches(person));
  if (found === undefined) {
    throw new Error(`The sample has nobody who is ${what}.`);
  }
  return found;
}

/** A Team member without a code app: signs in with the password alone. */
export function sampleTeamMember(): SamplePerson {
  return somebody(
    "a Team member without a code app",
    (person) => person.role === "team_member" && person.codeAppSecret === undefined,
  );
}

/** A person with a code app, and the secret of the app. */
export function sampleWithCodeApp(): { readonly person: SamplePerson; readonly secret: string } {
  const person = somebody(
    "signing in with a code app",
    (candidate) => candidate.codeAppSecret !== undefined,
  );
  return { person, secret: person.codeAppSecret ?? "" };
}

/** Puts a fresh copy of the sample's data folder into `folder`, which a server then starts on. */
export function copySampleInto(folder: string): void {
  requireSample();
  cpSync(join(sampleFolder(), layout.dataFolder), folder, { recursive: true });
}

/** The first administrator of the sample, who runs the demos (Ada Brandt). */
export function sampleAdministrator(): SamplePerson {
  return somebody("an Administrator", (person) => person.role === "administrator");
}

/** A person of the sample by name, for the people the demos name (Ana Moreno, Ben Hale). */
export function sampleNamed(name: string): SamplePerson {
  return somebody(`called ${name}`, (person) => person.name === name);
}

/** A row of *Who has which role* in the dump of the sample, as the dump writes it. */
export interface SampleRow {
  /** The address of the person, or the name of the group. */
  readonly who: string;
  /** The name of the role, or `null` for *No access*. */
  readonly role: string | null;
  /** The short name of the project, or `null` for the whole organization. */
  readonly project: string | null;
}

/** The rows of *Who has which role* the sample was made with, read from its dump. */
export function sampleRows(): readonly SampleRow[] {
  requireSample();
  const dump: unknown = JSON.parse(readFileSync(join(sampleFolder(), layout.dumpFile), "utf8"));
  const rows: unknown =
    typeof dump === "object" && dump !== null ? Reflect.get(dump, "assignments") : undefined;
  if (!Array.isArray(rows)) {
    throw new TypeError(`The dump ${layout.dumpFile} has no rows of Who has which role.`);
  }
  return rows.map((row: unknown) => {
    const read = (key: string): unknown =>
      typeof row === "object" && row !== null ? Reflect.get(row, key) : undefined;
    const who = read("who");
    const role = read("role");
    const project = read("project");
    return {
      who: typeof who === "string" ? who : "",
      role: typeof role === "string" ? role : null,
      project: typeof project === "string" ? project : null,
    };
  });
}

/** What the groups and custom roles of the sample are called, by their keys (`tools/dev/sample.json`). */
export const sampleNames: {
  readonly groups: Readonly<Record<string, string>>;
  readonly roles: Readonly<Record<string, string>>;
} = layout.organization;

/** A project of the sample. */
export interface SampleProject {
  readonly shortName: string;
  readonly name: string;
  /** The short name of the project it is inside, or `null` at the top level. */
  readonly inside: string | null;
}

/** A project as `tools/dev/sample.json` writes it. */
interface ProjectNames {
  readonly shortName: string;
  readonly name: string;
  readonly subProjects: readonly { readonly shortName: string; readonly name: string }[];
}

const projectNames: readonly ProjectNames[] = layout.organization.projects;

/** The projects of the sample: the top-level ones, then their sub-projects. */
export const sampleProjects: readonly SampleProject[] = [
  ...projectNames.map(({ shortName, name }) => ({ shortName, name, inside: null })),
  ...projectNames.flatMap((project) =>
    project.subProjects.map(({ shortName, name }) => ({
      shortName,
      name,
      inside: project.shortName,
    })),
  ),
];

/** What a project of the sample is called, by its short name. */
export function sampleProjectName(shortName: string): string {
  const named = sampleProjects.find((project) => project.shortName === shortName);
  if (named === undefined) {
    throw new Error(`The sample has no project ${shortName}.`);
  }
  return named.name;
}

/** A Guest who has the role in this project and no code app: signs in with the password alone. */
export function sampleGuestIn(project: string): SamplePerson {
  const rows = sampleRows().filter((row) => row.project === project && row.role === "Guest");
  return somebody(
    `a Guest in ${project}`,
    (person) => person.codeAppSecret === undefined && rows.some((row) => row.who === person.email),
  );
}
