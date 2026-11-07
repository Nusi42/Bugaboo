import { describe, expect, it } from "vitest";

import { entry, find, starters } from "./rows-testing.ts";
import { issueRows, rowTotal, startersOf } from "./rows.ts";

describe("time_04 the rows of time on issues", () => {
  it("makes one row for each issue, by its readable ID", () => {
    const rows = issueRows(
      [
        entry("i2", "WEB-141", "2026-10-05", 120),
        entry("i1", "WEB-9", "2026-10-06", 30),
        entry("i2", "WEB-141", "2026-10-06", 90),
      ],
      starters,
      find,
    );
    expect(rows.map((row) => row.readableId)).toEqual(["WEB-9", "WEB-141"]);
  });

  it("reads Activity and Cost center from the entries by the IDs of the starter fields", () => {
    const rows = issueRows([entry("i1", "WEB-1", "2026-10-05", 30)], starters, find);
    // The cost center keeps its number apart from its name, so a screen draws the number in the code font.
    expect(rows[0]).toMatchObject({
      activity: [{ shown_as: "Development" }],
      costCenter: [{ number: "4100", name: "Product development" }],
    });
  });

  it("names every value of a row whose entries differ", () => {
    const rows = issueRows(
      [entry("i1", "WEB-1", "2026-10-05", 30), entry("i1", "WEB-1", "2026-10-06", 30, "Testing")],
      starters,
      find,
    );
    expect(rows[0]).toMatchObject({
      activity: [{ shown_as: "Development" }, { shown_as: "Testing" }],
    });
  });
});

describe("time_04 the minutes of a row for each choice", () => {
  it("are added up for each choice, so a row with two says which is which", () => {
    const rows = issueRows(
      [
        entry("i1", "WEB-1", "2026-10-05", 30),
        entry("i1", "WEB-1", "2026-10-06", 45, "Testing"),
        entry("i1", "WEB-1", "2026-10-07", 15, "Testing"),
      ],
      starters,
      find,
    );
    expect(rows[0]?.choiceMinutes).toMatchObject({
      "id-of-Development": 30,
      "id-of-Testing": 60,
      "id-of-cost-center": 90,
    });
  });
});

describe("time_04 the totals of a cell and a row", () => {
  it("add up a cell and a row", () => {
    const rows = issueRows(
      [
        entry("i1", "WEB-1", "2026-10-05", 30),
        entry("i1", "WEB-1", "2026-10-05", 60),
        entry("i1", "WEB-1", "2026-10-06", 15),
        entry("i1", "WEB-1", "2026-10-12", 500),
      ],
      starters,
      find,
    );
    const days = ["2026-10-05", "2026-10-06"];
    expect(rows[0]?.entries["2026-10-05"]).toHaveLength(2);
    expect(rows.map((row) => rowTotal(row, days))).toEqual([105]);
  });
});

describe("time_04 the starter fields of the timer", () => {
  it("are none when the timer does not name them", () => {
    expect(startersOf({})).toEqual({ activity: undefined, costCenter: undefined });
  });
});
