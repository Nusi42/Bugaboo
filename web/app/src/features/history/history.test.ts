import type { Schema } from "@bugaboo/api-client";
import { describe, expect, it } from "vitest";

import {
  changeSentence,
  fieldLabel,
  groupByEvent,
  relationSentence,
  type HistoryEntry,
} from "./history.ts";
import { mentionBeingTyped, withMention } from "./mention-text.ts";

const ben = { shown: "name", id: "b", name: "Ben" } as const;
const ana = { shown: "name", id: "a", name: "Ana" } as const;

function change(overrides: Partial<Schema<"HistoryEntryItem">>): HistoryEntry {
  return { event: "e1", at: "2027-01-15T08:00:00Z", by: ben, kind: "change", ...overrides };
}

describe("iss_06 the history in plain sentences", () => {
  it("says what a field was and is, with the words of status and priority", () => {
    expect(
      changeSentence(change({ field: "priority", before_text: "normal", after_text: "critical" })),
    ).toBe("Ben changed Priority from Normal to Critical");
    expect(changeSentence(change({ field: "status", after_text: "in-progress" }))).toBe(
      "Ben set Status to In progress",
    );
  });

  it("names people the way the server showed them and says when a value was cleared", () => {
    expect(
      changeSentence(
        change({
          field: "assigned-to",
          before_people: [],
          after_people: [ana, { shown: "someone" }],
        }),
      ),
    ).toBe("Ben changed Assigned to from nobody to Ana, someone");
    expect(changeSentence(change({ field: "labels", before_text: "login" }))).toBe(
      "Ben cleared Labels",
    );
  });

  it("names an own field by the name the server sends", () => {
    expect(
      changeSentence(
        change({
          field: "0190f4c2-7a3e-7c1d-9b52-3d6e8f1a2b4c",
          field_name: "Severity",
          before_text: "Minor",
          after_text: "Critical",
        }),
      ),
    ).toBe("Ben changed Severity from Minor to Critical");
  });

  it("does not show what a long text held, only that it changed", () => {
    expect(changeSentence(change({ field: "description" }))).toBe("Ben changed Description");
  });
});

describe("iss_06 values the screen words itself", () => {
  it("words yes or no, a length of time and a number as the issue page does", () => {
    const own = { field: "0190f4c2-7a3e-7c1d-9b52-3d6e8f1a2b4c", field_name: "Billable" };
    expect(
      changeSentence(
        change({ ...own, field_kind: "yes-no", before_value: false, after_value: true }),
      ),
    ).toBe("Ben changed Billable from No to Yes");
    expect(changeSentence(change({ ...own, field_kind: "duration", after_value: 5400 }))).toBe(
      "Ben set Billable to 1h 30m",
    );
  });
});

describe("iss_06 relations and the items of one event", () => {
  it("names the parent and the relations by their readable IDs", () => {
    expect(fieldLabel("parent")).toBe("Part of");
    expect(
      relationSentence(
        change({
          kind: "relation",
          relation: "added",
          field: "blocks",
          issue: {
            id: "i2",
            key: "WEB-147",
            project: { id: "p", short_name: "WEB", name: "Website" },
            issue_type: { id: "t", name: "Bug", icon: "bug", retired: false },
          },
        }),
      ),
    ).toBe("Ben added a relation: this issue blocks WEB-147");
  });

  it("puts the fields of one event together and keeps a comment on its own", () => {
    const groups = groupByEvent([
      change({ field: "title" }),
      change({ field: "status" }),
      change({ event: "e2", kind: "comment" }),
      change({ event: "e3", field: "title" }),
    ]);
    expect(groups.map((group) => group.length)).toEqual([2, 1, 1]);
  });
});

describe("iss_05 the name being typed after an @", () => {
  it("finds the start of a name at the end of the text only", () => {
    expect(mentionBeingTyped("see @pro")).toBe("pro");
    expect(mentionBeingTyped("@")).toBe("");
    expect(mentionBeingTyped("mail ana@example.com")).toBeUndefined();
    expect(mentionBeingTyped("done @ana.silva and")).toBeUndefined();
  });

  it("puts the whole handle in its place, with a space after it", () => {
    expect(withMention("see @pro", "project.lead")).toBe("see @project.lead ");
    expect(withMention("@a", "ana.silva")).toBe("@ana.silva ");
  });
});
