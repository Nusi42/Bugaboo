import { describe, expect, it, vi } from "vitest";

import {
  askValuesOf,
  needsAsking,
  needsTheDescription,
  statusActions,
  type NeededField,
} from "./ask-for.ts";
import { emptyForm } from "./new-issue-form.ts";

const found = "0192b0f2-6f6e-7c11-8a41-2a8f0c3d7e51";
const testing = { key: "testing", name: "Testing", group: "in-progress" } as const;
const own: NeededField = { field: found, kind: "own" };
const people: NeededField = { field: "assigned-to", kind: "assigned-to" };
const description: NeededField = { field: "description", kind: "description" };

describe("wf_04 the status list names the status when two changes have the same words", () => {
  const react = {
    ask: vi.fn<(change: unknown) => void>(),
    make: vi.fn<(change: unknown) => void>(),
  };
  const done = { key: "done", name: "Done", group: "done" } as const;

  it("tells two changes with the same words apart by the status they lead to", () => {
    const labels = statusActions(
      [
        { to: testing, button: "Move on", needs: [] },
        { to: done, button: "Move on", needs: [] },
        { to: done, button: "Mark as done", needs: [] },
      ],
      react,
    ).map((action) => action.label);
    expect(labels).toEqual(["Move on (to Testing)", "Move on (to Done)", "Mark as done"]);
  });
});

describe("wf_04 the window of Ask for holds only the fields the change needs", () => {
  it("asks nothing for a change that needs nothing", () => {
    expect(needsAsking({ to: testing, button: "Send to testing", needs: [] })).toBe(false);
    expect(needsAsking({ to: testing, button: "Send to testing", needs: [own] })).toBe(true);
  });

  it("says when the description is needed, which is written on the issue and not asked for here", () => {
    expect(needsTheDescription([own, people])).toBe(false);
    expect(needsTheDescription([own, description])).toBe(true);
  });

  it("sends the status with only the values of the fields it asked for", () => {
    const filled = {
      ...emptyForm,
      title: "not asked for",
      assignedTo: [{ id: "p1", name: "Ana" }],
      fields: { [found]: "1.4.2", other: "not asked for" },
    };
    expect(askValuesOf(testing, [own], filled)).toEqual({
      status: testing,
      own: { [found]: "1.4.2" },
    });
    expect(askValuesOf(testing, [people], filled)).toEqual({
      status: testing,
      assignedTo: [{ id: "p1", name: "Ana" }],
    });
  });

  it("sends no value for a field that was left empty, so the server names it", () => {
    expect(askValuesOf(testing, [own], emptyForm)).toEqual({ status: testing });
  });
});
