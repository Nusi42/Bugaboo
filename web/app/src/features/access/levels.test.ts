import { describe, expect, it } from "vitest";

import { applied, isLevelTable, mayPick, type LevelTable } from "./levels.ts";

const levels = ["hidden", "can-see", "can-change"] as const;
type Level = (typeof levels)[number];

function table(project: LevelTable<string, Level>["project"]): LevelTable<string, Level> {
  return {
    project,
    roles: [
      { id: "r1", name: "Contributor" },
      { id: "r2", name: "Guest" },
    ],
    truncated: false,
    levels,
    rows: [
      {
        subject: "cost-center",
        yours: "can-change",
        cells: [
          { role: "r1", level: "can-see", origin: { kind: "role-own" } },
          { role: "r2", level: "hidden", origin: { kind: "role-own" } },
        ],
      },
    ],
  };
}

describe("sec_04 a level changes in the table before the server has answered", () => {
  it("shows the new level for the project that set it", () => {
    const project = { shown: "visible", id: "p", short_name: "WEB", name: "Website" } as const;
    const after = applied(table(project), { role: "r1", subject: "cost-center", level: "hidden" });
    expect(after?.rows[0]?.cells[0]).toEqual({
      role: "r1",
      level: "hidden",
      origin: { kind: "set-here" },
    });
    expect(after?.rows[0]?.cells[1]?.level).toBe("hidden");
  });

  it("shows the role's own level on the table of the organization", () => {
    const after = applied(table(null), { role: "r2", subject: "cost-center", level: "can-see" });
    expect(after?.rows[0]?.cells[1]).toEqual({
      role: "r2",
      level: "can-see",
      origin: { kind: "role-own" },
    });
  });

  it("shows nothing new for a setting that is taken away: the server says what counts then", () => {
    const before = table(null);
    expect(applied(before, { role: "r1", subject: "cost-center", level: null })).toBe(before);
    expect(
      applied(undefined, { role: "r1", subject: "cost-center", level: "hidden" }),
    ).toBeUndefined();
  });
});

describe("sec_04 nobody picks a level above their own", () => {
  it("allows the own level and below, and refuses above", () => {
    expect(mayPick(levels, "can-see", "hidden")).toBe(true);
    expect(mayPick(levels, "can-see", "can-see")).toBe(true);
    expect(mayPick(levels, "can-see", "can-change")).toBe(false);
  });

  it("knows a table from other things the cache may hold", () => {
    expect(isLevelTable(table(null))).toBe(true);
    expect(isLevelTable({ rows: [] })).toBe(false);
    expect(isLevelTable(null)).toBe(false);
  });
});
