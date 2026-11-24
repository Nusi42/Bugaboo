import { describe, expect, it } from "vitest";

import { differences, expectedFrom, goalPerson, type Reading } from "./judge.ts";
import { knownFrom } from "./known.ts";

/** An event of the dump: the label of its record and the newest value of each field. */
function event(entity: string, changes: Readonly<Record<string, unknown>>): unknown {
  return {
    entity,
    changes: Object.keys(changes).map((field) => ({ field, before: null, after: changes[field] })),
  };
}

const OPEN_DAY = { periods: [{ start: "2026-11-04T07:00:00Z", end: null }], breaks: [] };
const CLOSED_DAY = {
  periods: [{ start: "2026-11-04T07:00:00Z", end: "2026-11-04T15:00:00Z" }],
  breaks: [],
};

/** The parts of the dump of an old sample that the upgrade test reads. */
const dump = {
  start: "2026-11-17T18:00:00Z",
  people: [{ name: "Ada Brandt" }, { name: "Lea Fischer" }],
  issues: {
    projects: [{ total: 3 }, { total: 2 }],
    comments: 7,
    work: {
      time: { entries: 4, work_days: 6 },
      timeline: { person: { name: "Lea Fischer", away_times: 1 } },
      ai: { agents: [{ name: "Sorter" }, { name: "Fixer" }] },
    },
  },
  events: [
    event("issue#1", { title: "Same title", status: "open" }),
    event("issue#2", { title: "Price filter", status: "open" }),
    event("issue#3", { title: "Same title", status: "open" }),
    event("issue#4", { title: "Login screen", status: "open" }),
    event("issue#2", { status: "in-progress" }),
    event("comment#1", { issue: "issue#1", text: "On an issue with a title another has" }),
    event("comment#2", { issue: "issue#4", text: "First" }),
    event("comment#3", { issue: "issue#4", text: "Second" }),
    event("comment#2", { text: "First, edited" }),
    event("time_entry#1", { issue: "issue#2", date: "2026-11-04", minutes: 60 }),
    event("time_entry#2", { issue: "issue#2", date: "2026-11-04", minutes: 30 }),
    event("time_entry#3", { issue: "issue#2", date: "2026-11-05", minutes: 15 }),
    event("work_day#1", { person: "person#1", date: "2026-11-04", times: OPEN_DAY }),
    event("work_day#2", { person: "person#2", date: "2026-11-04", times: OPEN_DAY }),
    event("work_day#2", { times: CLOSED_DAY }),
  ],
};

const known = {
  issues: [
    { title: "Price filter", status: "in-progress" },
    { title: "Login screen", status: "open" },
  ],
  comments: { issue: "Login screen", texts: ["First, edited", "Second"] },
  timeEntries: { issue: "Price filter", date: "2026-11-04", minutes: [30, 60] },
  workDay: {
    person: "Lea Fischer",
    date: "2026-11-04",
    periods: [
      { start: Date.parse("2026-11-04T07:00:00Z"), end: Date.parse("2026-11-04T15:00:00Z") },
    ],
  },
};

const reading: Reading = {
  issues: 5,
  comments: 7,
  timeEntries: 4,
  workDays: 6,
  awayTimes: 1,
  ownThemes: 0,
  agents: ["Fixer", "Sorter"],
  known,
  backupBeforeUpgrade: "bugaboo-before-upgrade-20261119T055821Z-0.5.1.backup",
  backupFileThere: true,
};

describe("ops_06 the upgrade test holds the reading against the old dump", () => {
  it("good example: what the new program reads is what the old one wrote", () => {
    expect(differences(expectedFrom(dump), reading, "made")).toEqual([]);
    expect(goalPerson(dump)).toBe("Lea Fischer");
  });

  it("good example: with no migration due, no backup is made and none is asked for", () => {
    const none = { ...reading, backupBeforeUpgrade: undefined, backupFileThere: false };
    expect(differences(expectedFrom(dump), none, "none")).toEqual([]);
  });

  it("bad example: a backup made though no migration was due is named", () => {
    expect(differences(expectedFrom(dump), reading, "none")).toEqual([
      "the upgrade made the backup bugaboo-before-upgrade-20261119T055821Z-0.5.1.backup though no migration was due",
    ]);
  });

  it("bad example: a lost comment and a missing backup are named", () => {
    const found = differences(
      expectedFrom(dump),
      { ...reading, comments: 6, backupBeforeUpgrade: undefined },
      "made",
    );
    expect(found).toEqual([
      "comments: 7 before the upgrade, 6 after it",
      "the list of backups has no backup made by the upgrade",
    ]);
  });

  it("good example: the kept work days of a removed person are not among those read", () => {
    const removing = {
      ...dump,
      issues: {
        ...dump.issues,
        work: { ...dump.issues.work, privacy: { removed: { count: 1, work_days: 1 } } },
      },
    };
    expect(differences(expectedFrom(removing), { ...reading, workDays: 5 }, "made")).toEqual([]);
  });

  it("bad example: a dump without a count says which one it lacks", () => {
    expect(() =>
      expectedFrom({ ...dump, issues: { ...dump.issues, comments: undefined } }),
    ).toThrow("issues.comments");
  });
});

describe("ops_06 the known records of the old sample are found by their keys", () => {
  it("good example: the records of the dump with the newest value of each field", () => {
    expect(knownFrom(dump)).toEqual(known);
  });

  it("bad example: a record that reads otherwise after the upgrade is named", () => {
    const changed = {
      ...known,
      issues: [{ title: "Price filter", status: "open" }, ...known.issues.slice(1)],
      workDay: { ...known.workDay, periods: [] },
    };
    expect(differences(expectedFrom(dump), { ...reading, known: changed }, "made")).toEqual([
      `the known issues: ${JSON.stringify(known.issues)} before the upgrade, ${JSON.stringify(changed.issues)} after it`,
      `the known workDay: ${JSON.stringify(known.workDay)} before the upgrade, ${JSON.stringify(changed.workDay)} after it`,
    ]);
  });

  it("bad example: a dump without a finished work day says so", () => {
    expect(() => knownFrom({ ...dump, events: dump.events.slice(0, -1) })).toThrow(
      "finished work day",
    );
  });
});
