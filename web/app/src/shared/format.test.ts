import { describe, expect, it } from "vitest";

import { daysUntil, formatDate, splitUptime } from "./format.ts";

describe("ops_08 formatting for the status page", () => {
  it("splits an uptime into days, hours and minutes", () => {
    expect(splitUptime(0)).toEqual({ days: 0, hours: 0, minutes: 0 });
    expect(splitUptime(59)).toEqual({ days: 0, hours: 0, minutes: 0 });
    expect(splitUptime(3 * 86_400 + 4 * 3600 + 5 * 60 + 6)).toEqual({
      days: 3,
      hours: 4,
      minutes: 5,
    });
  });

  it("counts whole days until a moment, and goes negative after it", () => {
    const now = Date.parse("2026-10-05T12:00:00Z");
    expect(daysUntil("2026-10-15T12:00:00Z", now)).toBe(10);
    expect(daysUntil("2026-10-15T11:59:59Z", now)).toBe(9);
    expect(daysUntil("2026-10-04T12:00:00Z", now)).toBe(-1);
  });

  it("writes a day the way people say it", () => {
    expect(formatDate("2026-10-05T12:00:00Z")).toBe("October 5, 2026");
  });
});
