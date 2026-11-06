import type { Schema } from "@bugaboo/api-client";
import { describe, expect, it } from "vitest";

import { issueRows, rowTotal, startersOf } from "./rows.ts";

type Entry = Schema<"TimeEntryItem">;

function entry(
  issue: string,
  readable: string,
  date: string,
  minutes: number,
  activity = "Development",
): Entry {
  return {
    id: `${issue}-${date}-${minutes}`,
    issue,
    readable_id: readable,
    date,
    minutes,
    note: "",
    own_fields: { a1: `id-of-${activity}`, c1: "id-of-cost-center" },
    // The server says the names of the choices; the browser shows them as they come.
    own_field_names: { a1: [activity], c1: ["4100 Product development"] },
    version: 1,
  };
}

const starters = startersOf({ activity: "a1", "cost-center": "c1" });

describe("time_04 the rows of time on issues", () => {
  it("makes one row for each issue, by its readable ID", () => {
    const rows = issueRows(
      [
        entry("i2", "WEB-141", "2026-10-05", 120),
        entry("i1", "WEB-9", "2026-10-06", 30),
        entry("i2", "WEB-141", "2026-10-06", 90),
      ],
      starters,
    );
    expect(rows.map((row) => row.readableId)).toEqual(["WEB-9", "WEB-141"]);
  });

  it("reads Activity and Cost center from the entries by the IDs of the starter fields", () => {
    const rows = issueRows([entry("i1", "WEB-1", "2026-10-05", 30)], starters);
    expect(rows[0]).toMatchObject({
      activity: "Development",
      costCenter: "4100 Product development",
    });
  });

  it("names every value of a row whose entries differ", () => {
    const rows = issueRows(
      [entry("i1", "WEB-1", "2026-10-05", 30), entry("i1", "WEB-1", "2026-10-06", 30, "Testing")],
      starters,
    );
    expect(rows[0]).toMatchObject({ activity: "Development, Testing" });
  });

  it("adds up a cell and a row", () => {
    const rows = issueRows(
      [
        entry("i1", "WEB-1", "2026-10-05", 30),
        entry("i1", "WEB-1", "2026-10-05", 60),
        entry("i1", "WEB-1", "2026-10-06", 15),
        entry("i1", "WEB-1", "2026-10-12", 500),
      ],
      starters,
    );
    const days = ["2026-10-05", "2026-10-06"];
    expect(rows[0]?.entries["2026-10-05"]).toHaveLength(2);
    expect(rows.map((row) => rowTotal(row, days))).toEqual([105]);
  });

  it("has no starter fields when the timer does not name them", () => {
    expect(startersOf({})).toEqual({ activity: undefined, costCenter: undefined });
  });
});
