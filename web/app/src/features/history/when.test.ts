import type { Schema } from "@bugaboo/api-client";
import { describe, expect, it } from "vitest";

import { plainWords } from "../../shared/format.ts";
import { deviceRegion } from "../../shared/region.ts";
import { whenOf } from "./when.ts";

/** A row of time with the moment of its date, as the server sends it to a reader who may not see the moment. */
function timeRow(day?: string): Schema<"HistoryEntryItem"> {
  return {
    event: "e1",
    at: "2027-01-14T00:00:00Z",
    kind: "time",
    ...(day === undefined ? {} : { day }),
  };
}

describe("priv_09 a time entry is dated, not timed, for a reader who may not see the moment", () => {
  const west = {
    ...deviceRegion,
    timeZone: "America/New_York",
    dateFormat: "year-month-day",
  } as const;
  const east = { ...west, timeZone: "Pacific/Auckland" } as const;

  it("writes the date of the entry and no time, in any zone", () => {
    // The server sends the start of the date as the moment: that is 19:00 the day before in New York.
    const dated = timeRow("2027-01-14");
    expect(plainWords(whenOf(dated, west))).toBe("2027-01-14");
    expect(plainWords(whenOf(dated, east))).toBe("2027-01-14");
  });

  it("writes the moment, with its time, when no date is sent", () => {
    expect(whenOf(timeRow(), west)).toMatch(/19:00|7:00/u);
  });
});
