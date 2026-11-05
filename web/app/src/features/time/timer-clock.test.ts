import { describe, expect, it } from "vitest";

import { elapsedMinutes, elapsedSeconds } from "./timer-clock.ts";

describe("time_01 how long a timer has run", () => {
  it("counts from the moment the server says the timer started", () => {
    const started = "2027-01-14T08:00:00Z";
    const now = Date.parse("2027-01-14T08:42:08Z");
    expect(elapsedSeconds(started, 0, now)).toBe(2528);
    expect(elapsedMinutes(started, 0, now)).toBe(42);
  });

  it("uses the clock of the server when the clock of the browser is off", () => {
    const started = "2027-01-14T08:00:00Z";
    // The browser is three minutes behind the server: its 08:39:30 is 08:42:30 for the server.
    const now = Date.parse("2027-01-14T08:39:30Z");
    const offset = 3 * 60 * 1000;
    expect(elapsedMinutes(started, offset, now)).toBe(42);
  });

  it("never goes below zero, also for a start that is a moment ahead", () => {
    const now = Date.parse("2027-01-14T08:00:00Z");
    expect(elapsedSeconds("2027-01-14T08:00:05Z", 0, now)).toBe(0);
    expect(elapsedSeconds("not a moment", 0, now)).toBe(0);
  });
});
