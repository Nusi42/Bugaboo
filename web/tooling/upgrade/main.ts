import { readFileSync } from "node:fs";
import { join } from "node:path";

import layout from "../../../tools/dev/sample.json" with { type: "json" };
import { signInThroughApi } from "../../app/e2e/support/accounts.ts";
import { peopleIn, type SamplePerson } from "../../app/e2e/support/sample.ts";
import { startServer } from "../../app/e2e/support/server.ts";
import { runDriver } from "../driver.ts";
import { readPorts } from "../ports.ts";
import { differences, expectedFrom, goalPerson, sampleEnd } from "./judge.ts";
import { readAfterUpgrade } from "./read.ts";

// `cargo xtask upgrade-test` (architecture §11, "The upgrade test", TEST-UPGRADE): xtask made the
// sample organization with the program of the previous milestone in the folder `--sample`; this
// driver starts the program of HEAD on its data folder (`BB_SERVER_BIN`), which makes the backup
// before the upgrade and runs every migration or refuses to start, signs in as an administrator
// of the sample, holds what the API reads against what the old program wrote down, and checks
// that the security log verifies. On PostgreSQL xtask passes the address of the database in the
// variable of the sample's layout.

/** The acceptance test this run is. */
const TEST_NAME = "ops_06_the_sample_of_the_previous_milestone_reads_the_same_after_the_upgrade";

/** The value that follows a flag. */
function flag(name: string): string {
  const given = process.argv.slice(2);
  const value = given[given.indexOf(name) + 1];
  if (!given.includes(name) || value === undefined) {
    throw new TypeError(`${name} is missing.`);
  }
  return value;
}

/** The settings that point the server at the database of the run (SQLite is the default). */
function databaseSettings(database: string): Readonly<Record<string, string>> {
  if (database === "sqlite") {
    return {};
  }
  const url = process.env[layout.databaseUrlVariable];
  if (url === undefined || url === "") {
    throw new TypeError(
      `A run on PostgreSQL needs the address of the database in ${layout.databaseUrlVariable}.`,
    );
  }
  return { BUGABOO_DATABASE_URL: url };
}

/** Somebody of the sample with `role` who signs in without a code app. */
function personWithRole(people: readonly SamplePerson[], role: string): SamplePerson {
  const found = people.find((person) => person.role === role && person.codeAppSecret === undefined);
  if (found === undefined) {
    throw new Error(`The sample has nobody with the role ${role} and no code app.`);
  }
  return found;
}

async function main(): Promise<void> {
  const sample = flag("--sample");
  const database = flag("--database");
  const dump: unknown = JSON.parse(readFileSync(join(sample, layout.dumpFile), "utf8"));
  const withRole = (role: string): SamplePerson => personWithRole(peopleIn(sample), role);
  const settings = databaseSettings(database);
  const server = await startServer(
    readPorts().upgradeTestServer,
    `upgrade-${database}`,
    settings,
    false,
    join(sample, layout.dataFolder),
  );
  try {
    const administrator = await signInThroughApi(server, withRole("administrator"));
    const hr = await signInThroughApi(server, withRole("hr"));
    const expected = expectedFrom(dump);
    const reading = await readAfterUpgrade(
      { administrator: administrator.client, hr: hr.client },
      sampleEnd(dump),
      goalPerson(dump),
      expected.known,
    );
    await administrator.dispose();
    await hr.dispose();
    const found = differences(expected, reading);
    if (found.length > 0) {
      throw new Error(`${TEST_NAME} on ${database}:\n${found.join("\n")}`);
    }
    process.stdout.write(
      `${TEST_NAME} on ${database}: ${String(reading.issues)} issues, ${String(reading.comments)} comments, ` +
        `${String(reading.timeEntries)} time entries, ${String(reading.workDays)} work days, ` +
        `${String(reading.agents.length)} agents and the known records read the same after the upgrade; it made the backup ` +
        `${reading.backupBeforeUpgrade ?? ""} first.\n`,
    );
  } finally {
    await server.stop();
  }
  // The command that verifies the log finds the database as the server did: by its settings.
  Object.assign(process.env, settings);
  process.stdout.write(`${await server.verifySecurityLog()}\n`);
}

await runDriver(main);
