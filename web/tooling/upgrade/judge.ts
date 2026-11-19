import { type KnownRecords, knownFrom } from "./known.ts";

// What the upgrade test holds against what (architecture §11, "The upgrade test", TEST-UPGRADE): the
// counts the program of the previous milestone wrote into the dump of its sample and the known
// records worked out from its events (`known.ts`), and what the program of HEAD reads through the
// API after the upgrade. Pure, so its tests need no server.

/** What the API read after the upgrade. */
export interface Reading {
  readonly issues: number;
  readonly comments: number;
  readonly timeEntries: number;
  readonly workDays: number;
  /** The away times of the person of the goal in the timeline, by name. */
  readonly awayTimes: number;
  readonly ownThemes: number;
  /** The names of the agents, sorted. */
  readonly agents: readonly string[];
  /** A few records, each found by its key. */
  readonly known: KnownRecords;
  /** The name of the backup the upgrade made, when the list of backups has its record. */
  readonly backupBeforeUpgrade: string | undefined;
  /** Whether that backup is among the files of the backups folder. */
  readonly backupFileThere: boolean;
}

/** What the old program wrote, as the reading names it. */
export type Expected = Omit<Reading, "backupBeforeUpgrade" | "backupFileThere">;

/** Every count and list the old program wrote, in the order the differences are named. */
const COMPARED = [
  "issues",
  "comments",
  "timeEntries",
  "workDays",
  "awayTimes",
  "ownThemes",
  "agents",
] as const satisfies readonly (keyof Expected)[];

/** Every known record, in the order the differences are named. */
const KNOWN = [
  "issues",
  "comments",
  "timeEntries",
  "workDay",
] as const satisfies readonly (keyof KnownRecords)[];

/** The value at `path` in the dump, or a plain message that the old dump has no such value. */
function at(dump: unknown, path: readonly string[]): unknown {
  let value = dump;
  for (const part of path) {
    if (typeof value !== "object" || value === null || !(part in value)) {
      throw new TypeError(`The dump of the previous milestone has no ${path.join(".")}.`);
    }
    value = Reflect.get(value, part);
  }
  return value;
}

function numberAt(dump: unknown, path: readonly string[]): number {
  const value = at(dump, path);
  if (typeof value !== "number") {
    throw new TypeError(`${path.join(".")} of the dump of the previous milestone is no number.`);
  }
  return value;
}

function listAt(dump: unknown, path: readonly string[]): readonly unknown[] {
  const value = at(dump, path);
  if (!Array.isArray(value)) {
    throw new TypeError(`${path.join(".")} of the dump of the previous milestone is no list.`);
  }
  return value;
}

/** The person of the goal in the timeline, whose away times are read. */
export function goalPerson(dump: unknown): string {
  const name = at(dump, ["issues", "work", "timeline", "person", "name"]);
  if (typeof name !== "string") {
    throw new TypeError("The dump of the previous milestone names no person of the goal.");
  }
  return name;
}

/** The moment the sample ends, from the dump, in milliseconds since 1970. */
export function sampleEnd(dump: unknown): number {
  const start = at(dump, ["start"]);
  if (typeof start !== "string") {
    throw new TypeError("The dump of the previous milestone has no moment.");
  }
  return Date.parse(start);
}

/**
 * What the dump of the old sample says. The sample of a milestone that makes no own themes has no
 * count of them, which is none.
 */
export function expectedFrom(dump: unknown): Expected {
  const work = ["issues", "work"];
  const counted = at(dump, work);
  const themes: unknown =
    typeof counted === "object" && counted !== null
      ? Reflect.get(counted, "own_themes")
      : undefined;
  return {
    issues: listAt(dump, ["issues", "projects"])
      .map((project) => numberAt(project, ["total"]))
      .reduce((sum, count) => sum + count, 0),
    comments: numberAt(dump, ["issues", "comments"]),
    timeEntries: numberAt(dump, [...work, "time", "entries"]),
    workDays: numberAt(dump, [...work, "time", "work_days"]),
    awayTimes: numberAt(dump, [...work, "timeline", "person", "away_times"]),
    ownThemes: typeof themes === "number" ? themes : 0,
    agents: listAt(dump, [...work, "ai", "agents"])
      .map((agent) => {
        const name = at(agent, ["name"]);
        return typeof name === "string" ? name : "";
      })
      .toSorted(),
    known: knownFrom(dump),
  };
}

/** What differs between what the old program wrote and what the new one reads: one line each. */
export function differences(expected: Expected, reading: Reading): readonly string[] {
  const found: string[] = [];
  for (const key of COMPARED) {
    const before = JSON.stringify(expected[key]);
    const after = JSON.stringify(reading[key]);
    if (before !== after) {
      found.push(`${key}: ${before} before the upgrade, ${after} after it`);
    }
  }
  for (const key of KNOWN) {
    const before = JSON.stringify(expected.known[key]);
    const after = JSON.stringify(reading.known[key]);
    if (before !== after) {
      found.push(`the known ${key}: ${before} before the upgrade, ${after} after it`);
    }
  }
  if (reading.backupBeforeUpgrade === undefined) {
    found.push("the list of backups has no backup made by the upgrade");
  } else if (!reading.backupFileThere) {
    found.push(`the backup ${reading.backupBeforeUpgrade} is not in the backups folder`);
  }
  return found;
}
