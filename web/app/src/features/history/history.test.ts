import type { Schema } from "@bugaboo/api-client";
import { describe, expect, it } from "vitest";

import {
  changeSentence,
  groupByEvent,
  relationSentence,
  sentenceOf,
  type HistoryEntry,
} from "./history.ts";
import { mentionBeingTyped, withMention } from "./mention-text.ts";

const ben = { shown: "name", id: "b", name: "Ben" } as const;
const ana = { shown: "name", id: "a", name: "Ana" } as const;

function change(overrides: Partial<Schema<"HistoryEntryItem">>): HistoryEntry {
  return { event: "e1", at: "2027-01-15T08:00:00Z", by: ben, kind: "change", ...overrides };
}

describe("iss_06 the history lists choices", () => {
  it("joins the names of the choices of a pick field", () => {
    expect(
      changeSentence(
        change({
          field: "0190f4c2-7a3e-7c1d-9b52-3d6e8f1a2b4c",
          field_name: "Browsers",
          before_names: ["Firefox"],
          after_names: ["Firefox", "Safari"],
        }),
      ),
    ).toBe("Ben changed Browsers from Firefox to Firefox and Safari");
  });
});

describe("iss_06 the history of a field of people reads as a sentence", () => {
  it("reads a field of people that was first set without a doubled to (M3 review)", () => {
    // The name of the field already ends in a preposition: *set Assigned to to nobody* is not a sentence.
    expect(changeSentence(change({ field: "assigned-to", after_people: [] }))).toBe(
      "Ben set Assigned to: nobody",
    );
    expect(changeSentence(change({ field: "assigned-to", after_people: [ana] }))).toBe(
      "Ben set Assigned to: Ana",
    );
  });
});

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
    ).toBe("Ben changed Assigned to from nobody to Ana and someone");
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
    ).toBe("Ben added a relation: this issue · blocks · WEB-147");
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

/** The sentence of a file item of the history with this event. */
function fileWords(file_event: Schema<"FileEvent"> | undefined, file_name?: string): string {
  return sentenceOf(
    change({
      kind: "file",
      ...(file_event === undefined ? {} : { file_event }),
      ...(file_name === undefined ? {} : { file_name }),
    }),
  );
}

describe("iss_06 files in the history (M4-F50)", () => {
  it("says who added a file, deleted it and brought it back, with its name", () => {
    expect(fileWords("added", "jam.txt")).toBe("Ben added the file jam.txt");
    expect(fileWords("deleted", "jam.txt")).toBe("Ben deleted the file jam.txt");
    expect(fileWords("restored", "jam.txt")).toBe("Ben brought back the file jam.txt");
  });

  it("says a file when the file is gone for good, and never reads an unknown event as added", () => {
    expect(fileWords("deleted")).toBe("Ben deleted a file");
    expect(fileWords("added")).toBe("Ben added a file");
    expect(fileWords(undefined, "jam.txt")).toBe("Ben changed a file");
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

/** A project as the history names it. */
function project(name: string) {
  return { id: name, short_name: name, name };
}

describe("tag_01 tags, labels and a move in the history", () => {
  it("says a tag was set when there were none, and cleared when the last one went", () => {
    expect(
      changeSentence(change({ field: "tags", before_names: [], after_names: ["login"] })),
    ).toBe("Ben set Tags to login");
    expect(
      changeSentence(change({ field: "tags", before_names: ["login"], after_names: [] })),
    ).toBe("Ben cleared Tags");
  });

  it("names the project an issue moved from and to", () => {
    expect(
      changeSentence(
        change({
          field: "project",
          before_project: project("Website"),
          after_project: project("Shop"),
        }),
      ),
    ).toBe("Ben changed Project from Website to Shop");
  });
});
