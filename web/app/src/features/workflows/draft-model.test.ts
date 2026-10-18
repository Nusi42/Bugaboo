import type { Schema } from "@bugaboo/api-client";
import { describe, expect, it } from "vitest";

import {
  addChange,
  addStatus,
  bodyOf,
  changesFrom,
  draftOf,
  moveStatus,
  removeStatus,
  renameStatus,
  sameDraft,
  setButton,
  type Draft,
} from "./draft-model.ts";
import { noRules } from "./rules-model.ts";

const rules: Schema<"StatusRulesBody"> = {
  who: { roles: [], groups: [], assigned: false, reported_by: false },
  only_if: [],
  must_fill: [],
  ask_for: [],
};

const editor: Schema<"WorkflowEditorBody"> = {
  name: "Simple",
  statuses: [
    { key: "open", name: "Open", group: "not-started" },
    { key: "in-progress", name: "In progress", group: "in-progress" },
    { key: "done", name: "Done", group: "done" },
  ],
  changes: [
    { from: "open", to: "in-progress", button: "Start working", rules },
    { from: "in-progress", to: "done", button: "Mark as done", rules },
  ],
  roles: [],
  groups: [],
  fields: [],
};

/** The status with this name, or a stop when the draft has none. */
function named(draft: Draft, name: string) {
  const found = draft.statuses.find((status) => status.name === name);
  if (found === undefined) {
    throw new TypeError(`The draft has no status ${name}.`);
  }
  return found;
}

/** The ID of the first status change of the draft, or a stop when it has none. */
function firstChange(draft: Draft): string {
  const found = draft.changes[0];
  if (found === undefined) {
    throw new TypeError("The draft has no status change.");
  }
  return found.id;
}

/** The keys of the statuses as *Publish* would send them; a new status has none. */
function sentKeys(draft: Draft): (string | null | undefined)[] {
  const keys: (string | null | undefined)[] = [];
  for (const each of bodyOf(draft).statuses) {
    keys.push(each.key);
  }
  return keys;
}

/** The status changes as *Publish* would send them, as `from>to`. */
function sentWays(draft: Draft): string[] {
  const ways: string[] = [];
  for (const each of bodyOf(draft).changes) {
    ways.push(`${each.from}>${each.to}`);
  }
  return ways;
}

function names(draft: Draft): string[] {
  return draft.statuses.map((status) => status.name);
}

describe("wf_02 the draft of a workflow is what the editor showed until it is changed", () => {
  it("sends back exactly what the editor showed when nothing was changed", () => {
    const draft = draftOf(editor);
    expect(sameDraft(draft, draftOf(editor))).toBe(true);
    expect(sentKeys(draft)).toEqual(["open", "in-progress", "done"]);
    expect(sentWays(draft)).toEqual(["open>in-progress", "in-progress>done"]);
  });

  it("keeps the key of a status that is renamed", () => {
    const draft = renameStatus(draftOf(editor), "open", "To do");
    expect(bodyOf(draft).statuses[0]).toEqual({ key: "open", name: "To do", group: "not-started" });
    expect(sameDraft(draft, draftOf(editor))).toBe(false);
  });

  it("changes the words of a button", () => {
    const draft = setButton(draftOf(editor), firstChange(draftOf(editor)), "Begin");
    expect(bodyOf(draft).changes[0]).toMatchObject({ button: "Begin" });
  });
});

describe("wf_02 a status is added, moved and removed in plain steps", () => {
  it("adds a status without a key and names it in the changes by its name", () => {
    const withStatus = addStatus(draftOf(editor), "Testing", "in-progress");
    const draft = addChange(
      withStatus,
      named(withStatus, "In progress").id,
      named(withStatus, "Testing").id,
      "Send to testing",
    );
    const body = bodyOf(draft);
    expect(named(draft, "Testing").key).toBeNull();
    expect(body.statuses.at(-1)).toEqual({ name: "Testing", group: "in-progress" });
    expect(body.changes.at(-1)).toMatchObject({ from: "in-progress", to: "Testing" });
  });

  it("names a new status in the changes by its new name when it is renamed", () => {
    const withStatus = addStatus(draftOf(editor), "Testing", "in-progress");
    const added = addChange(withStatus, "open", named(withStatus, "Testing").id, "Send to testing");
    const draft = renameStatus(added, named(added, "Testing").id, "In test");
    expect(bodyOf(draft).changes.at(-1)).toMatchObject({ to: "In test" });
  });

  it("takes the changes of a removed status with it", () => {
    const draft = removeStatus(draftOf(editor), "in-progress");
    expect(names(draft)).toEqual(["Open", "Done"]);
    expect(draft.changes).toHaveLength(0);
  });

  it("moves a status up or down and not past the ends", () => {
    const draft = draftOf(editor);
    expect(names(moveStatus(draft, "done", -1))).toEqual(["Open", "Done", "In progress"]);
    expect(names(moveStatus(draft, "open", -1))).toEqual(["Open", "In progress", "Done"]);
    expect(names(moveStatus(draft, "done", 1))).toEqual(["Open", "In progress", "Done"]);
  });
});

describe("wf_02 a status change that makes no sense is not added", () => {
  it("adds what was typed and leaves the judging to Publish, which the server does", () => {
    const draft = draftOf(editor);
    expect(addStatus(draft, "open", "done").statuses).toHaveLength(4);
    expect(addStatus(draft, "  Testing ", "done").statuses.at(-1)?.name).toBe("Testing");
    expect(changesFrom(addChange(draft, "open", "done", " Mark as done "), "open")).toHaveLength(2);
    expect(addChange(draft, "open", "done", " Mark as done ").changes.at(-1)?.button).toBe(
      "Mark as done",
    );
  });

  it("gives a new status change rules that name nobody and ask for nothing", () => {
    const draft = addChange(draftOf(editor), "open", "done", "Mark as done");
    expect(draft.changes.at(-1)?.rules).toEqual(noRules);
  });
});
