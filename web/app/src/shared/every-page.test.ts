import { describe, expect, it } from "vitest";

import { readingEveryPage } from "./every-page.ts";

describe("ux_01 a list read page by page holds its screen only while it reads", () => {
  it("reads while its first page is on its way, and while more pages are to come", () => {
    expect(readingEveryPage({ isLoading: true, hasNextPage: false, isError: false })).toBe(true);
    expect(readingEveryPage({ isLoading: false, hasNextPage: true, isError: false })).toBe(true);
  });

  it("is done once every page is there", () => {
    expect(readingEveryPage({ isLoading: false, hasNextPage: false, isError: false })).toBe(false);
  });

  it("is done when a page could not be read, so the screen shows and says why", () => {
    expect(readingEveryPage({ isLoading: false, hasNextPage: true, isError: true })).toBe(false);
    expect(readingEveryPage({ isLoading: false, hasNextPage: false, isError: true })).toBe(false);
  });
});
