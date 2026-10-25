import { describe, expect, it } from "vitest";

import { basicFields, formViewOf } from "./layouts.ts";

describe("cfg_01 the form a person gets", () => {
  it("shows everything while the layout is on the way, so nothing flashes away", () => {
    const view = formViewOf({ data: undefined, isError: false }, true);
    expect(view).toMatchObject({ ready: false, failed: false });
    expect(view.shows("priority")).toBe(true);
  });

  it("waits for nothing when there is no project to ask the layout about", () => {
    const view = formViewOf({ data: undefined, isError: false }, false);
    expect(view).toMatchObject({ ready: true, failed: false, sections: [] });
    expect(view.shows("priority")).toBe(true);
  });

  it("shows what the layout lists and nothing else", () => {
    const view = formViewOf(
      {
        data: {
          kind: "new-issue",
          sections: [{ tab: "", title: "", fields: ["description"] }],
        },
        isError: false,
      },
      true,
    );
    expect(view).toMatchObject({ ready: true, failed: false });
    expect(view.shows("description")).toBe(true);
    expect(view.shows("priority")).toBe(false);
  });

  it("shows only the basics when the layout could not be read, never everything", () => {
    const view = formViewOf({ data: undefined, isError: true }, true);
    expect(view).toMatchObject({ ready: true, failed: true });
    expect(view.sections.flatMap((section) => section.fields)).toEqual(basicFields);
    expect(view.shows("description")).toBe(true);
    for (const field of ["priority", "status", "assigned-to", "reported-by", "an-own-field"]) {
      expect(view.shows(field)).toBe(false);
    }
  });
});
