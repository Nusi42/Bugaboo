import { leaseSeconds, secondsPerMinute } from "@bugaboo/api-client";

import layout from "../../../tools/dev/load.json" with { type: "json" };

/**
 * The seconds at the end of a run in which nobody starts to edit under a lock: the lease, and ten
 * seconds for a slow answer. Every lease has ended when the run does, so a lock that is still held
 * then is one that outlived its lease.
 */
export const QUIET_SECONDS = leaseSeconds + 10;

/** The shortest run: there is time to edit before the quiet seconds, and the people meet. */
const SHORTEST_SECONDS = QUIET_SECONDS + 15;

/** Which database the server of a run uses. */
export type LoadDatabase = "sqlite" | "postgres";

/** What a run is asked to do (`cargo xtask load`). */
export interface LoadOptions {
  /** How many people work at the same time. */
  readonly people: number;
  /** How long they work, in seconds. */
  readonly seconds: number;
  readonly database: LoadDatabase;
  /** The number the choices of the people are made from, so a failure can be run again. */
  readonly seed: number;
  /** The data folder of the server, which is left in place for the check after the run. */
  readonly dataDir: string;
  /** The file the book of the run is written to. */
  readonly bookFile: string;
}

/** The name of the variable that carries the address of the PostgreSQL database (tools/dev/load.json). */
export const DATABASE_URL_VARIABLE = layout.databaseVariable;

/** The value that follows a flag, or a plain message about the flag. */
function valueOf(flags: readonly string[], name: string): string | undefined {
  const at = flags.indexOf(name);
  return at < 0 ? undefined : flags[at + 1];
}

/** A whole number flag with a default. */
function numberOf(flags: readonly string[], name: string, fallback: number): number {
  const text = valueOf(flags, name);
  const number = text === undefined ? fallback : Number(text);
  if (!Number.isInteger(number) || number < 1) {
    throw new TypeError(`${name} needs a whole number of 1 or more, not ${text ?? "nothing"}.`);
  }
  return number;
}

/** A text flag that must be there. */
function requiredOf(flags: readonly string[], name: string): string {
  const text = valueOf(flags, name);
  if (text === undefined || text === "") {
    throw new TypeError(`${name} needs a value: the check after the run reads what it names.`);
  }
  return text;
}

/**
 * The options from the arguments `cargo xtask load` passes: `--people`, `--seconds`, `--database`,
 * `--seed`, `--data-dir` and `--book`.
 */
export function parseOptions(flags: readonly string[]): LoadOptions {
  const database = valueOf(flags, "--database") ?? "sqlite";
  if (database !== "sqlite" && database !== "postgres") {
    throw new TypeError(`--database is sqlite or postgres, not ${database}.`);
  }
  const seconds = numberOf(flags, "--seconds", layout.minutes * secondsPerMinute);
  if (seconds < SHORTEST_SECONDS) {
    throw new TypeError(
      `--seconds is at least ${String(SHORTEST_SECONDS)}: the last ${String(QUIET_SECONDS)} are for the locks to run out.`,
    );
  }
  return {
    people: numberOf(flags, "--people", layout.people),
    seconds,
    database,
    seed: numberOf(flags, "--seed", 42),
    dataDir: requiredOf(flags, "--data-dir"),
    bookFile: requiredOf(flags, "--book"),
  };
}
