import { readFileSync } from "node:fs";
import { join } from "node:path";

import layout from "../../../../tools/dev/sample.json" with { type: "json" };
import { samplePeople, sampleFolder } from "./sample.ts";

/** An agent of the sample: its name, the e-mail address of the person it works for and its key. */
export interface SampleAgent {
  readonly name: string;
  readonly email: string;
  readonly key: string;
}

/**
 * The agents of the sample with their keys, from the comment lines of `sign-in.txt` that start with
 * the mark of the plan (`agentKeyMark`): agent, e-mail address and key, separated by tabs.
 */
export function sampleAgents(): readonly SampleAgent[] {
  // Fails with what to do when the sample was not made.
  samplePeople();
  return agentsIn(sampleFolder());
}

/** The agents of a sample or of a large dataset made in `folder`, from its `sign-in.txt`. */
export function agentsIn(folder: string): readonly SampleAgent[] {
  const mark = `${layout.issues.work.ai.agentKeyMark}\t`;
  return readFileSync(join(folder, layout.signInFile), "utf8")
    .split("\n")
    .map((line) => line.replace(/\r$/u, ""))
    .filter((line) => line.startsWith(mark))
    .map((line) => {
      const [name, email, key] = line.slice(mark.length).split("\t");
      if (name === undefined || email === undefined || key === undefined) {
        throw new Error(`A line of an agent in ${layout.signInFile} has no three columns.`);
      }
      return { name, email, key };
    });
}
