import { rmSync } from "node:fs";

import { ownId } from "../../app/e2e/support/accounts.ts";
import { backUp } from "../../app/e2e/support/operations-scene.ts";
import { timed } from "../../app/e2e/support/perf.ts";
import { backupIn } from "../../app/e2e/support/server.ts";
import { type Context, type Scenario } from "./scenarios.ts";

// The scenarios of privacy and operations on the large dataset (M9, PERF-01), registered in the list
// of `scenarios.ts`, which spreads this one: the walk of the data of a person with years of it, and a
// backup of the whole dataset.

const RUNS = 200;
/** A backup takes minutes on the large dataset: a few runs, none thrown away. */
const BACKUP_RUNS = 3;

/**
 * The walk of the data of a person with years of it (PRIV-04): what *Remove a person* counts before
 * its window opens, for the person with the agents of the dataset.
 */
async function personDataWalk(context: Context): Promise<number> {
  const person = await ownId(context.agent.person);
  return (
    await timed(() =>
      context.session.client.GET("/api/v1/people/{person}/removal-plan", {
        params: { path: { person } },
      }),
    )
  ).ms;
}

/**
 * A backup of the whole dataset asked for through the route, as *Create a backup now* does
 * (OPS-07), from the ask until the list says it is made. The file is deleted after each run, so the
 * dataset does not grow with the runs.
 */
async function backupLarge(context: Context): Promise<number> {
  const made = await timed(() => backUp(context.session));
  rmSync(backupIn(context.agent.server.dataDir, made.value));
  return made.ms;
}

export const privacyScenarios: readonly Scenario[] = [
  {
    // What *Remove a person* counts before its window opens (PRIV-04): within the budget of a page.
    name: "person-data-walk",
    budget: "page",
    runs: RUNS,
    once: personDataWalk,
  },
  {
    // The backup of the 1 M dataset (OPS-07): its length is written down, no budget yet.
    name: "backup-large",
    budget: "backup",
    runs: BACKUP_RUNS,
    warmUp: 0,
    once: backupLarge,
  },
];
