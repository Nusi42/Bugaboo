import { describe, expect, it } from "vitest";

import { listTemplate } from "./list-template.ts";

describe("ux_4_2 the columns of the list as one template", () => {
  it("lets the columns of fields give up room, down to the least, before the title", () => {
    const template = listTemplate([
      { column: "title" },
      { column: "person" },
      { column: "changed" },
    ]);
    expect(template).toBe(
      "minmax(var(--size-col-title), 1fr) minmax(var(--size-col-least), var(--size-col-person)) minmax(var(--size-col-least), var(--size-col-changed))",
    );
  });

  it("keeps the width of the priority, the ID and an icon", () => {
    expect(listTemplate([{ column: "priority" }, { column: "key" }, { column: "status" }])).toBe(
      "var(--size-col-priority) var(--size-col-key) var(--size-col-icon)",
    );
  });

  it("takes the width a person gave a column, in pixels", () => {
    expect(listTemplate([{ column: "person", width: 150 }])).toBe("150px");
  });
});
