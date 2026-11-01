import { describe, expect, it } from "vitest";

import { timeSentence } from "./time-sentence.ts";
import { testWording } from "./wording-testing.ts";

const ana = { shown: "name", id: "a", name: "Ana" } as const;

/** The sentence of time with this event and this length, booked by Ana. */
function said(time_event: "booked" | "changed" | "deleted", minutes: number): string {
  return timeSentence(testWording, { by: ana, time_event, minutes });
}

describe("time_02 time booked on an issue in the history", () => {
  it("says who booked how long, changed or deleted it, in the short form of a length", () => {
    expect(said("booked", 90)).toBe("Ana booked 1h 30m");
    expect(said("changed", 45)).toBe("Ana changed booked time to 45m");
    expect(said("deleted", 120)).toBe("Ana deleted booked time of 2h");
  });

  it("says only that time was booked to a reader who may not read Time spent", () => {
    expect(timeSentence(testWording, { by: ana, time_event: "booked" })).toBe("Ana booked time");
  });
});
