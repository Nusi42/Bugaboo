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

  it("makes the ID as wide as the longest one drawn, and no narrower than its head needs", () => {
    expect(listTemplate([{ column: "key" }], 19)).toBe(
      "max(var(--size-col-key), calc(var(--size-code-char) * 19))",
    );
    expect(listTemplate([{ column: "key" }], 0)).toBe("var(--size-col-key)");
    expect(listTemplate([{ column: "key", width: 150 }], 19)).toBe("150px");
  });
});
