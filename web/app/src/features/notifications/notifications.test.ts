import { markSafeHtml, type Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { describe, expect, it } from "vitest";

import {
  groupsOf,
  projectSettingsOf,
  rowsOf,
  sentencesOf,
  targetOf,
  type NotificationItem,
  type NotificationRow,
} from "./notifications.ts";

const ben = { shown: "name", id: "b", name: "Ben" } as const;
const issue = (key: string) =>
  ({
    id: key,
    key,
    title: "A title",
    project: { id: "p", short_name: "WEB", name: "Web" },
    issue_type: { id: "t", name: "Task", icon: "bug", retired: false },
  }) as const;

function entry(overrides: Partial<NotificationItem>): NotificationItem {
  return {
    id: "n1",
    kind: "followed_activity",
    at: "2027-01-15T08:00:00Z",
    read: false,
    issue: issue("WEB-1"),
    by: ben,
    ...overrides,
  };
}

/** The one notification the items make. */
function rowOfItems(...items: readonly NotificationItem[]): NotificationRow {
  const [row] = rowsOf(items);
  if (row === undefined) {
    throw new TypeError("The items make no notification.");
  }
  return row;
}

/** The sentences of the one notification the items make. */
function say(...items: readonly NotificationItem[]): readonly string[] {
  return sentencesOf(rowOfItems(...items));
}

describe("ntf_01 the items of one notification are one row", () => {
  it("joins the items that share an ID and keeps the order of the list", () => {
    const rows = rowsOf([
      entry({ id: "a", field: "priority" }),
      entry({ id: "a", field: "status" }),
      entry({ id: "b", comment: "c1", text: "Hi", html: markSafeHtml("<p>Hi</p>") }),
    ]);
    expect(rows.map((row) => [row.id, row.entries.length])).toEqual([
      ["a", 2],
      ["b", 1],
    ]);
  });
});

describe("ntf_01 the rows are grouped by issue, the issue with the newest row first", () => {
  it("puts the rows of one issue together where the issue first appears", () => {
    const rows = rowsOf([
      entry({ id: "1", issue: issue("WEB-1") }),
      entry({ id: "2", issue: issue("WEB-2") }),
      entry({ id: "3", issue: issue("WEB-1") }),
    ]);
    const groups = groupsOf(rows);
    expect(groups.map((group) => [group.key, group.rows.map((row) => row.id)])).toEqual([
      ["WEB-1", ["1", "3"]],
      ["WEB-2", ["2"]],
    ]);
  });

  it("keeps a notification without an issue in a group of its own", () => {
    const groups = groupsOf(rowsOf([entry({ id: "1", issue: null, kind: "run_ended" })]));
    expect(groups).toHaveLength(1);
    expect(groups[0]?.issue).toBeNull();
  });
});

describe("ntf_01 Open goes to the exact comment or change", () => {
  it("goes to the comment of a comment, and to the moment of a change", () => {
    const comment = rowOfItems(entry({ comment: "c1", text: "Hi" }));
    const change = rowOfItems(entry({ field: "priority", after_text: "high" }));
    expect(targetOf(comment)).toEqual({
      kind: "comment",
      comment: "c1",
    });
    expect(targetOf(change)).toEqual({
      kind: "change",
      at: "2027-01-15T08:00:00Z",
    });
  });

  it("goes to the issue alone when there is nothing more exact", () => {
    expect(targetOf(rowOfItems(entry({ kind: "assigned" })))).toBeUndefined();
  });
});

describe("ntf_01 a row says in a sentence what happened", () => {
  it("names who mentioned, assigned or wrote", () => {
    expect(say(entry({ kind: "mentioned", comment: "c" }))).toEqual([
      t("notifications.mentioned", { who: "Ben" }),
    ]);
    expect(say(entry({ kind: "assigned" }))).toEqual([t("notifications.assigned", { who: "Ben" })]);
    // The change that assigned the issue is said as an assignment, not as a change of a field.
    expect(say(entry({ kind: "assigned", field: "assigned-to", after_people: [ben] }))).toEqual([
      t("notifications.assigned", { who: "Ben" }),
    ]);
    expect(say(entry({ comment: "c", text: "Hi" }))).toEqual([
      t("notifications.commented", { who: "Ben" }),
    ]);
  });

  it("words each changed field like the history does", () => {
    const sentences = say(
      entry({ id: "a", field: "priority", before_text: "normal", after_text: "high" }),
      entry({ id: "a", field: "status", before_text: "open", after_text: "done" }),
    );
    expect(sentences).toHaveLength(2);
  });

  it("says who asked to finish editing, and how a run ended", () => {
    expect(
      say(entry({ kind: "edit_notice", notice: "finish_asked", part: "description" })),
    ).toEqual([t("notifications.finish-asked", { who: "Ben" })]);
    expect(
      say(entry({ kind: "run_ended", issue: null, by: null, run: "r", run_done: true })),
    ).toEqual([t("notifications.run-done")]);
    expect(
      say(entry({ kind: "run_ended", issue: null, by: null, run: "r", run_done: false })),
    ).toEqual([t("notifications.run-failed")]);
  });
});

describe("m4_f144 a renewal of issue IDs that gave up names the project", () => {
  it("names a project the person may name, and says so for one they cannot see", () => {
    expect(
      say(
        entry({
          kind: "run_ended",
          issue: null,
          by: null,
          issue_ids_failed: { shown: "visible", id: "p", name: "Website" },
        }),
      ),
    ).toEqual([t("notifications.issue-ids-failed", { project: "Website" })]);
    expect(
      say(
        entry({
          kind: "run_ended",
          issue: null,
          by: null,
          issue_ids_failed: { shown: "hidden" },
        }),
      ),
    ).toEqual([t("notifications.issue-ids-failed-hidden")]);
  });

  it("says who added a file, with its name when the file is still there", () => {
    expect(
      say(entry({ kind: "followed_activity", file_event: "added", file_name: "jam.txt" })),
    ).toEqual([t("history.file-added", { who: "Ben", name: "jam.txt" })]);
    expect(say(entry({ kind: "followed_activity", file_event: "added" }))).toEqual([
      t("history.file-added-unnamed", { who: "Ben" }),
    ]);
  });
});

const failed = (project: Schema<"ProjectLabelBody">) =>
  rowOfItems(entry({ kind: "run_ended", issue: null, by: null, issue_ids_failed: project }));

describe("m4_f144 the notice of a renewal that gave up leads to the settings of its project", () => {
  it("goes to the settings of a project the person may name, and nowhere for a hidden one", () => {
    expect(
      projectSettingsOf(failed({ shown: "visible", id: "p", name: "Website", short_name: "WEB" })),
    ).toBe("/p/WEB/settings");
    expect(projectSettingsOf(failed({ shown: "hidden" }))).toBeUndefined();
  });
});
