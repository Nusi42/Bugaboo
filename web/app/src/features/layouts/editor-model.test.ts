import { describe, expect, it } from "vitest";

import {
  addSection,
  bodyOf,
  editSection,
  nudgeField,
  nudgeSection,
  placeField,
  removeSection,
  sameLayout,
  sectionOf,
  setRole,
  unplaceField,
  type Draft,
  type Row,
} from "./editor-model.ts";

const everyone = ["team", "guest"];

function row(field: string, roles: readonly string[] = everyone): Row {
  return { field, name: null, roles, limitedFor: [], required: false };
}

const start: Draft = {
  sections: [
    { key: "a", tab: "", title: "Main", rows: [row("description"), row("priority")] },
    { key: "b", tab: "", title: "More", rows: [] },
  ],
  available: [row("due-date", [])],
};

function fieldsOf(draft: Draft, key: string): string[] {
  return (draft.sections.find((section) => section.key === key)?.rows ?? []).map(
    (each) => each.field,
  );
}

/** The key of the section at a place. */
function keyAt(draft: Draft, at: number): string {
  return draft.sections[at]?.key ?? "";
}

describe("cfg_01 the fields of a layout in the editor", () => {
  it("moves a field into another section and keeps its roles", () => {
    const next = placeField(start, "priority", { section: "b", index: 0 }, everyone);
    expect(fieldsOf(next, "a")).toEqual(["description"]);
    expect(fieldsOf(next, "b")).toEqual(["priority"]);
    expect(next.sections[1]?.rows[0]?.roles).toEqual(everyone);
  });

  it("adds an available field with every role ticked, then unticking Guest keeps Team member", () => {
    const added = placeField(start, "due-date", { section: "b", index: 0 }, everyone);
    expect(added.available).toEqual([]);
    expect(added.sections[1]?.rows[0]?.roles).toEqual(everyone);
    const narrowed = setRole(added, "due-date", "guest", false);
    expect(narrowed.sections[1]?.rows[0]?.roles).toEqual(["team"]);
    expect(setRole(narrowed, "due-date", "guest", true).sections[1]?.rows[0]?.roles).toEqual([
      "team",
      "guest",
    ]);
  });

  it("moves a field up and down inside its section, and not past the ends", () => {
    const down = nudgeField(start, "description", 1);
    expect(fieldsOf(down, "a")).toEqual(["priority", "description"]);
    expect(fieldsOf(nudgeField(down, "description", 1), "a")).toEqual(["priority", "description"]);
    expect(fieldsOf(nudgeField(down, "description", -1), "a")).toEqual(["description", "priority"]);
  });

  it("takes a field off the form and lists it with the others that are not on it", () => {
    const next = unplaceField(start, "priority");
    expect(fieldsOf(next, "a")).toEqual(["description"]);
    expect(next.available.map((each) => each.field)).toEqual(["due-date", "priority"]);
    expect(next.available[1]?.roles).toEqual([]);
    expect(sectionOf(next, "priority")).toBeUndefined();
  });
});

describe("cfg_01 the sections of a layout in the editor", () => {
  it("adds, renames, moves and removes sections; a removed section's fields are available again", () => {
    const added = addSection(start);
    const key = keyAt(added, 2);
    expect(added.sections).toHaveLength(3);
    const named = editSection(added, key, { title: "Extra", tab: "Details" });
    expect(named.sections[2]).toMatchObject({ title: "Extra", tab: "Details" });
    expect(nudgeSection(named, key, -1).sections.map((section) => section.title)).toEqual([
      "Main",
      "Extra",
      "More",
    ]);
    const removed = removeSection(start, "a");
    expect(removed.sections.map((section) => section.key)).toEqual(["b"]);
    expect(removed.available.map((each) => each.field)).toEqual([
      "due-date",
      "description",
      "priority",
    ]);
  });

  it("says what the API takes, and whether anything changed", () => {
    expect(bodyOf(start)[0]).toEqual({
      tab: "",
      title: "Main",
      fields: [
        { field: "description", roles: everyone },
        { field: "priority", roles: everyone },
      ],
    });
    expect(sameLayout(start, start)).toBe(true);
    expect(sameLayout(start, nudgeField(start, "description", 1))).toBe(false);
  });
});
