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
