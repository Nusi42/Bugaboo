import { describe, expect, it } from "vitest";

import { testSearchLanguage } from "./search-language-testing.ts";
import { withGroupFilter, withProject } from "./with-filter.ts";

const language = testSearchLanguage;

describe("the filters a list adds to a search", () => {
  it("joins the filter a group carries to the search with and", () => {
    expect(withGroupFilter(language, "status is Open", 'assignedTo is "Ana Bell"')).toBe(
      'status is Open and assignedTo is "Ana Bell"',
    );
    expect(withGroupFilter(language, "", "assignedTo is empty")).toBe("assignedTo is empty");
  });

  it("keeps the order of the search", () => {
    expect(withGroupFilter(language, "sort by title, A to Z", "priority is high")).toBe(
      "priority is high sort by title, A to Z",
    );
  });

  it("puts a project in the text of a search saved from a project's list", () => {
    expect(withProject(language, "status is Open", "WEB")).toBe(
      "status is Open and project within WEB",
    );
  });
});
