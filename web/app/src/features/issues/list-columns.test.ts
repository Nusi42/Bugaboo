import { describe, expect, it } from "vitest";

import { chooseColumns, moveColumn, projectColumns } from "./list-columns.ts";

describe("cfg_02 the columns a person chooses in Customize", () => {
  it("starts with the columns of UX 4.2 and no tick box or buttons among the choices", () => {
    expect(projectColumns).toEqual([
      "status",
      "key",
      "type",
      "title",
      "labels",
      "priority",
      "person",
      "changed",
    ]);
  });

  it("keeps the ID and the title when everything is taken away", () => {
    expect(chooseColumns([])).toEqual(["key", "title"]);
  });

  it("keeps the order the person gave and leaves out what is not a choice", () => {
    expect(chooseColumns(["title", "priority", "select", "key", "labels"])).toEqual([
      "title",
      "priority",
      "key",
      "labels",
    ]);
  });

  it("moves a column up and down and stops at the ends", () => {
    expect(moveColumn(projectColumns, "type", "up").slice(0, 3)).toEqual(["status", "type", "key"]);
    expect(moveColumn(projectColumns, "status", "up")).toEqual(projectColumns);
    expect(moveColumn(projectColumns, "changed", "down")).toEqual(projectColumns);
  });
});
