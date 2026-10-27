import { describe, expect, it } from "vitest";

import {
  cellOf,
  chooseColumns,
  isDrawnColumn,
  moveColumn,
  projectColumns,
} from "./list-columns.ts";

const readable = ["title", "status", "priority", "labels", "plannedFor"];

describe("the columns a person chooses in Customize", () => {
  it("starts with the columns of UX 4.2 as search words, with no tick box or buttons", () => {
    expect(projectColumns).toEqual([
      "status",
      "id",
      "type",
      "title",
      "labels",
      "priority",
      "assignedTo",
      "changed",
    ]);
  });

  it("draws a search word in the cell of its field, and a field without a cell as a plain value", () => {
    expect(cellOf("id")).toBe("key");
    expect(cellOf("assignedTo")).toBe("person");
    expect(cellOf("plannedFor")).toBe("value");
  });

  it("keeps the ID and the title when everything is taken away", () => {
    expect(chooseColumns([], readable)).toEqual(["id", "title"]);
  });

  it("keeps the order the person gave, a field of their own, and once each", () => {
    expect(
      chooseColumns(["title", "priority", "plannedFor", "id", "priority", "labels"], readable),
    ).toEqual(["title", "priority", "plannedFor", "id", "labels"]);
  });

  it("leaves out what is no readable field, such as the tick box or a hidden field", () => {
    expect(chooseColumns(["select", "secret", "title", "action"], readable)).toEqual([
      "id",
      "title",
    ]);
  });

  it("moves a column up and down and stops at the ends", () => {
    expect(moveColumn(projectColumns, "type", "up").slice(0, 3)).toEqual(["status", "type", "id"]);
    expect(moveColumn(projectColumns, "status", "up")).toEqual(projectColumns);
    expect(moveColumn(projectColumns, "changed", "down")).toEqual(projectColumns);
  });
});

const field = (kind: string, choices: readonly string[] = []) => ({ kind, choices });

describe("the columns that can be drawn", () => {
  it("leaves out a field of the organization that holds an ID, and one that is not readable", () => {
    expect(isDrawnColumn("reviewer", field("person"))).toBe(false);
    expect(isDrawnColumn("relatedIssue", field("issue"))).toBe(false);
    expect(isDrawnColumn("owner", field("project"))).toBe(false);
    // A group of people reaches the search as a choice field that lists no choices.
    expect(isDrawnColumn("team", field("choice"))).toBe(false);
    expect(isDrawnColumn("secret")).toBe(false);
  });

  it("keeps the fields that have a cell of their own and the plain values", () => {
    expect(isDrawnColumn("assignedTo", field("person"))).toBe(true);
    expect(isDrawnColumn("project", field("project"))).toBe(true);
    expect(isDrawnColumn("plannedFor", field("choice", ["2.4"]))).toBe(true);
    expect(isDrawnColumn("estimate", field("duration"))).toBe(true);
  });
});
