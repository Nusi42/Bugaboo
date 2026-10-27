import { describe, expect, it } from "vitest";

import { cellOf, chooseColumns, moveColumn, projectColumns } from "./list-columns.ts";

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
