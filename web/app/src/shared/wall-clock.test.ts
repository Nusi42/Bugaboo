import { describe, expect, it } from "vitest";

import { deviceRegion, type Region } from "./region.ts";
import { momentOnDay, readTimeOfDay, timeBoxText } from "./wall-clock.ts";

const berlin: Region = { ...deviceRegion, timeZone: "Europe/Berlin" };

describe("time_08 a time of day somebody types", () => {
  it("reads the usual ways to write one", () => {
    expect(readTimeOfDay("8:02")).toEqual({ hour: 8, minute: 2 });
    expect(readTimeOfDay("08:02")).toEqual({ hour: 8, minute: 2 });
    expect(readTimeOfDay("8.02")).toEqual({ hour: 8, minute: 2 });
    expect(readTimeOfDay("1630")).toEqual({ hour: 16, minute: 30 });
    expect(readTimeOfDay("16h30")).toEqual({ hour: 16, minute: 30 });
    expect(readTimeOfDay("9")).toEqual({ hour: 9, minute: 0 });
    expect(readTimeOfDay(" 17:45 ")).toEqual({ hour: 17, minute: 45 });
  });

  it("reads the twelve-hour clock", () => {
    expect(readTimeOfDay("8 pm")).toEqual({ hour: 20, minute: 0 });
    expect(readTimeOfDay("8:02 AM")).toEqual({ hour: 8, minute: 2 });
    expect(readTimeOfDay("12:15 am")).toEqual({ hour: 0, minute: 15 });
    expect(readTimeOfDay("12:15 pm")).toEqual({ hour: 12, minute: 15 });
  });

  it("refuses what is no time of day", () => {
    for (const text of ["", "soon", "24:00", "8:75", "13 pm", "0 am", "8:2", "8m", "8.", "8 a"]) {
      expect(readTimeOfDay(text)).toBeUndefined();
    }
  });
});

describe("time_08 a moment on a day in the zone of the person", () => {
  it("is the moment the clock of the zone shows that time", () => {
    // Berlin is one hour ahead of UTC in January and two in July.
    const winter = momentOnDay("2027-01-14", { hour: 8, minute: 30 }, berlin);
    const summer = momentOnDay("2027-07-14", { hour: 8, minute: 30 }, berlin);
    expect(new Date(Number(winter)).toISOString()).toBe("2027-01-14T07:30:00.000Z");
    expect(new Date(Number(summer)).toISOString()).toBe("2027-07-14T06:30:00.000Z");
  });

  it("is none for a day that does not exist, and is written back as the box wrote it", () => {
    expect(momentOnDay("2027-02-30", { hour: 8, minute: 0 }, berlin)).toBeNull();
    const moment = momentOnDay("2027-01-14", { hour: 8, minute: 2 }, berlin);
    expect(timeBoxText(Number(moment), berlin)).toBe("08:02");
  });
});
