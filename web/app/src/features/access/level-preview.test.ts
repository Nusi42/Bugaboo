import { describe, expect, it } from "vitest";

import type { LevelTable } from "./levels.ts";
import { previewLines } from "./level-preview.ts";

/** Words of the test, not text for people. */
const guest = { id: "r1", name: "Guest" };

const table: LevelTable<string, string> = {
  levels: ["hidden", "role-only", "names"],
  roles: [guest],
  rows: [
    {
      subject: "assigned-to",
      yours: "names",
      cells: [{ role: guest.id, level: "role-only", origin: { kind: "role-own" } }],
    },
    {
      subject: "authors",
      yours: "names",
      cells: [{ role: guest.id, level: "hidden", origin: { kind: "role-own" } }],
    },
    {
      subject: "editing",
      yours: "names",
      cells: [{ role: guest.id, level: "hidden", origin: { kind: "role-own" } }],
    },
  ],
  truncated: false,
};

describe("priv_12 and sec_04 the preview of what a role sees", () => {
  it("names every row, level by level from the highest, and leaves out a level the role has nowhere", () => {
    const lines = previewLines({
      table,
      role: guest.id,
      subjectName: (subject) => subject,
      sentence: (level, subjects) => `${level}: ${subjects}`,
    });
    expect(lines.map((line) => line.level)).toEqual(["role-only", "hidden"]);
    expect(lines[1]?.text).toContain("authors");
    expect(lines[1]?.text).toContain("editing");
  });
});
