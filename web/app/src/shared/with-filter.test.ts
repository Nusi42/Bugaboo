import { describe, expect, it } from "vitest";

import { testSearchLanguage } from "./search-language-testing.ts";
import { withFilter, withProject } from "./with-filter.ts";

const language = testSearchLanguage;

describe("the filters a list adds to a search", () => {
  it("adds the filter of a group, quoted where the language needs it", () => {
    expect(withFilter(language, "status is Open", "assignedTo", "Ana Bell")).toBe(
      'status is Open and assignedTo is "Ana Bell"',
    );
    expect(withFilter(language, "", "assignedTo", { empty: true })).toBe("assignedTo is empty");
  });

  it("keeps the order of the search", () => {
    expect(withFilter(language, "sort by title, A to Z", "priority", "high")).toBe(
      "priority is high sort by title, A to Z",
    );
  });

  it("puts a project in the text of a search saved from a project's list", () => {
    expect(withProject(language, "status is Open", "WEB")).toBe(
      "status is Open and project within WEB",
    );
  });
});
