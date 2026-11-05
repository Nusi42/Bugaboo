import { describe, expect, it } from "vitest";

import { deviceRegion, setRegion } from "../../shared/region.ts";
import { endOfOpenDay } from "./work-day-end.ts";

describe("time_08 the end of a day that was left open", () => {
  it("is read by the clock of the zone of the day, whatever the Region says now", () => {
    setRegion({ ...deviceRegion, timeZone: "Europe/Berlin" });
    const tokyo = endOfOpenDay({ date: "2027-01-14", zone: "Asia/Tokyo" }, "17:30");
    const berlin = endOfOpenDay({ date: "2027-01-14", zone: "Europe/Berlin" }, "17:30");
    // 17:30 in Tokyo (UTC+9) is 08:30 UTC, and 17:30 in Berlin (UTC+1) is 16:30 UTC.
    expect(new Date(Number(tokyo)).toISOString()).toBe("2027-01-14T08:30:00.000Z");
    expect(new Date(Number(berlin)).toISOString()).toBe("2027-01-14T16:30:00.000Z");
  });

  it("is none for text that is no time of day", () => {
    expect(endOfOpenDay({ date: "2027-01-14", zone: "Europe/Berlin" }, "soon")).toBeNull();
  });
});
