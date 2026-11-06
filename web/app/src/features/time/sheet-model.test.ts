import { describe, expect, it } from "vitest";

import { readBreaks } from "./sheet-model.ts";

describe("time_08 breaks are typed as times", () => {
  it("reads one break and several breaks", () => {
    expect(readBreaks("12:10-12:40")).toEqual({ value: [[730, 760]] });
    expect(readBreaks("10:00-10:15, 12:10-12:40")).toEqual({
      value: [
        [600, 615],
        [730, 760],
      ],
    });
  });

  it("refuses a plain total and says to type the times", () => {
    expect(readBreaks("30m")).toHaveProperty("mistake");
    expect(readBreaks("0:30")).toHaveProperty("mistake");
  });

  it("refuses a break that is not two times", () => {
    expect(readBreaks("12:10-lunch")).toHaveProperty("mistake");
  });

  it("takes an empty cell as no breaks", () => {
    expect(readBreaks("  ")).toEqual({ empty: true });
  });
});
