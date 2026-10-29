import { describe, expect, it } from "vitest";

import { listNeeds, listTemplate } from "./list-template.ts";

describe("ux_4_2 the columns of the list as one template", () => {
  it("lets the columns of fields give up room, down to the least, before the title", () => {
    const template = listTemplate([
      { column: "title" },
      { column: "person" },
      { column: "changed" },
    ]);
    expect(template).toBe(
      "minmax(var(--size-col-title-least), 1fr) minmax(var(--size-col-least), calc(var(--size-col-person) + var(--size-target-compact))) minmax(var(--size-col-least), calc(var(--size-col-changed) + var(--size-target-compact)))",
    );
  });

  it("keeps the width of the ID and an icon, and gives the priority its head: the name and the arrow", () => {
    expect(listTemplate([{ column: "priority" }, { column: "key" }, { column: "status" }])).toBe(
      "calc(var(--size-col-priority) + var(--size-icon-small)) var(--size-col-key) var(--size-col-icon)",
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

  it("counts the room the columns need as they are drawn: the widths a person gave", () => {
    // No page here, so a token is 0 px: only what is given counts.
    expect(
      listNeeds([
        { column: "title", width: 200 },
        { column: "person", width: 150 },
      ]),
    ).toBe(350);
    expect(listNeeds([{ column: "key" }, { column: "title" }], 8)).toBe(0);
  });
});
