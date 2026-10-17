import { describe, expect, it } from "vitest";

import { askedFieldsOf, askValuesOf, needsAsking } from "./ask-for.ts";
import { emptyForm } from "./new-issue-form.ts";

const found = "0192b0f2-6f6e-7c11-8a41-2a8f0c3d7e51";
const testing = { key: "testing", name: "Testing", group: "in-progress" } as const;

describe("wf_04 the window of Ask for holds only the fields the change needs", () => {
  it("asks nothing for a change that needs nothing", () => {
    expect(needsAsking({ to: testing, button: "Send to testing", needs: [] })).toBe(false);
    expect(needsAsking({ to: testing, button: "Send to testing", needs: [found] })).toBe(true);
  });

  it("tells the kinds of field apart", () => {
    expect(askedFieldsOf(["description", "assigned-to", found])).toEqual([
      { kind: "description" },
      { kind: "assigned-to" },
      { kind: "own", field: found },
    ]);
  });

  it("sends the status with only the values of the fields it asked for", () => {
    const filled = {
      ...emptyForm,
      title: "not asked for",
      assignedTo: [{ id: "p1", name: "Ana" }],
      fields: { [found]: "1.4.2", other: "not asked for" },
    };
    expect(askValuesOf(testing, [found], filled)).toEqual({
      status: testing,
      own: { [found]: "1.4.2" },
    });
    expect(askValuesOf(testing, ["assigned-to"], filled)).toEqual({
      status: testing,
      assignedTo: [{ id: "p1", name: "Ana" }],
    });
  });

  it("sends no value for a field that was left empty, so the server names it", () => {
    expect(askValuesOf(testing, [found], emptyForm)).toEqual({ status: testing });
  });
});
