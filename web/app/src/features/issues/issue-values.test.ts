import { describe, expect, it } from "vitest";

import { after, undoSentence, isDetail, valuesBefore } from "./issue-values.ts";
import type { IssueDetail } from "./slots.ts";

const ana = { shown: "name", id: "u1", name: "Ana" } as const;

const open = { key: "open", name: "Open", group: "not-started" } as const;
const inProgress = { key: "in-progress", name: "In progress", group: "in-progress" } as const;

const detail: IssueDetail = {
  issue: {
    id: "i1",
    key: "WEB-142",
    title: "Login fails",
    status: open,
    priority: "normal",
    assigned_to: [ana],
    version: 4,
  },
  own_fields_html: [],
  may_change: true,
  may_delete: false,
  changeable: ["title", "status", "priority", "assigned-to"],
};

describe("ux_06 a change shows at once and counts the version up by one", () => {
  it("changes the fields that were named and nothing else", () => {
    const next = after(detail, { values: { priority: "high" }, version: 4 });
    expect(next?.issue.priority).toBe("high");
    expect(next?.issue.status).toEqual(open);
    expect(next?.issue.title).toBe("Login fails");
    expect(next?.issue.version).toBe(5);
  });

  it("shows the people it is assigned to by name until the server answers", () => {
    const ben = { id: "u2", name: "Ben" };
    const next = after(detail, { values: { assignedTo: [ben] }, version: 4 });
    expect(next?.issue.assigned_to).toEqual([{ shown: "name", id: "u2", name: "Ben" }]);
    expect(after(undefined, { values: {}, version: 1 })).toBeUndefined();
  });

  it("knows the values to put back for Undo, only for the fields the change touched", () => {
    const back = valuesBefore(detail, { priority: "critical", assignedTo: [] });
    expect(back).toEqual({ priority: "normal", assignedTo: [{ id: "u1", name: "Ana" }] });
  });

  it("recognizes the page of an issue in the cache and nothing else", () => {
    expect(isDetail(detail)).toBe(true);
    expect(isDetail({ items: [] })).toBe(false);
    expect(isDetail(null)).toBe(false);
  });
});

describe("ux_06 the sentence that goes with Undo", () => {
  it("says the field and the words of the new value", () => {
    expect(undoSentence({ priority: "high" })).toBe("Priority changed to High.");
    expect(undoSentence({ status: inProgress })).toBe("Status changed to In progress.");
    expect(undoSentence({ title: "New title" })).toBe("Title changed to New title.");
  });

  it("names the people, or says the field is cleared", () => {
    expect(undoSentence({ assignedTo: [{ id: "u1", name: "Ana" }] })).toBe(
      "Assigned to changed to Ana.",
    );
    expect(undoSentence({ assignedTo: [] })).toBe("Assigned to cleared.");
  });
});
