import { describe, expect, it } from "vitest";

import { clockText, elapsedSeconds } from "./timer-clock.ts";

describe("time_01 the face of a running timer", () => {
  it("counts from the moment the server says the timer started", () => {
    const started = "2027-01-14T08:00:00Z";
    const now = Date.parse("2027-01-14T08:00:42Z");
    expect(elapsedSeconds(started, 0, now)).toBe(42);
  });

  it("uses the clock of the server when the clock of the browser is off", () => {
    const started = "2027-01-14T08:00:00Z";
    // The browser is three minutes behind the server: its 07:57:42 is 08:00:42 for the server.
    const now = Date.parse("2027-01-14T07:57:42Z");
    const offset = 3 * 60 * 1000;
    expect(elapsedSeconds(started, offset, now)).toBe(42);
  });

  it("never goes below zero, also for a start that is a moment ahead", () => {
    const now = Date.parse("2027-01-14T08:00:00Z");
    expect(elapsedSeconds("2027-01-14T08:00:05Z", 0, now)).toBe(0);
    expect(elapsedSeconds("not a moment", 0, now)).toBe(0);
  });

  it("writes minutes and seconds under an hour and hours from an hour", () => {
    expect(clockText(0)).toBe("0:00");
    expect(clockText(42)).toBe("0:42");
    expect(clockText(61)).toBe("1:01");
    expect(clockText(3599)).toBe("59:59");
    expect(clockText(3600)).toBe("1:00:00");
    expect(clockText(3600 + 5 * 60 + 9)).toBe("1:05:09");
    expect(clockText(26 * 3600)).toBe("26:00:00");
  });
});
