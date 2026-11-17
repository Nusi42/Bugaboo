import { describe, expect, it } from "vitest";

import { isOwnSheet } from "./own-sheet.ts";

describe("thm_04 the address of the sheet of an own theme", () => {
  it("takes the server's sheet of a saved theme and of a draft", () => {
    expect(isOwnSheet("/themes/01a1e6d6-9b79-724f-9337-da69be204b26.css?v=2")).toBe(true);
    expect(isOwnSheet("/themes/theme-1.css")).toBe(true);
    expect(isOwnSheet("/themes/theme-1/draft.css?v=12")).toBe(true);
  });

  it("refuses any other address, also one that only starts like a sheet", () => {
    for (const address of [
      "/themes/../api/v1/me.css",
      "/themes/%2e%2e/api/v1/me.css",
      "/themes/theme-1/../../api/v1/me",
      "//elsewhere.test/themes/x.css",
      "https://elsewhere.test/themes/x.css",
      "javascript:alert(1)",
      "/themes/theme-1.css?v=2&x=1",
      "/themes/theme-1.js",
      "/api/v1/themes/theme-1.css",
      "",
    ]) {
      expect.soft(isOwnSheet(address)).toBe(false);
    }
  });
});
