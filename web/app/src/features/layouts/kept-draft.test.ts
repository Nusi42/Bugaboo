import type { Schema } from "@bugaboo/api-client";
import { describe, expect, it } from "vitest";

import { differsFromLayout, restoredDraft } from "./kept-draft.ts";

function field(key: string, roles: readonly string[]): Schema<"EditorFieldBody"> {
  return { field: key, name: null, roles, limited_for: [], required: false };
}

const editor: Schema<"LayoutEditorBody"> = {
  kind: "new-issue",
  layout: { id: "l-1", version: 3 },
  sections: [{ tab: "", title: "Theirs", fields: [field("description", ["team"])] }],
  available: [field("priority", []), field("assigned-to", [])],
  roles: [{ id: "team", name: "Team member" }],
  choose_roles: true,
};

describe("cfg_01 a kept draft on the layout as it is now", () => {
  it("brings back the sections, the order and the ticked roles that still exist", () => {
    const kept = JSON.stringify([
      {
        tab: "",
        title: "Mine",
        fields: [
          { field: "priority", roles: ["team", "gone"] },
          { field: "description", roles: [] },
          { field: "removed-since", roles: ["team"] },
        ],
      },
    ]);
    const draft = restoredDraft(editor, kept);
    expect(draft?.sections.map((section) => section.title)).toEqual(["Mine"]);
    expect(draft?.sections[0]?.rows.map((each) => [each.field, each.roles])).toEqual([
      ["priority", ["team"]],
      ["description", []],
    ]);
    // What the draft does not hold is listed with the fields that are not on the form.
    expect(draft?.available.map((each) => each.field)).toEqual(["assigned-to"]);
  });

  it("keeps the roles as they are for somebody who may not choose roles", () => {
    const blind = { ...editor, choose_roles: false, roles: [] };
    const kept = JSON.stringify([
      { tab: "", title: "", fields: [{ field: "priority", roles: ["team"] }] },
    ]);
    expect(restoredDraft(blind, kept)?.sections[0]?.rows[0]?.roles).toEqual([]);
  });

  it("is nothing for a text that is no draft of sections", () => {
    expect(restoredDraft(editor, "not json")).toBeUndefined();
    expect(restoredDraft(editor, '{"a":1}')).toBeUndefined();
    expect(restoredDraft(editor, '[{"tab":1}]')).toBeUndefined();
  });
});

describe("cfg_01 a kept draft is offered when it differs from the layout", () => {
  const same = JSON.stringify([
    { tab: "", title: "Theirs", fields: [{ field: "description", roles: ["team"] }] },
  ]);
  const other = JSON.stringify([
    { tab: "", title: "Mine", fields: [{ field: "description", roles: [] }] },
  ]);

  it("is offered when it differs, whatever version it started from", () => {
    expect(differsFromLayout(editor, other)).toBe(true);
  });

  it("is not offered when it holds what the layout shows", () => {
    expect(differsFromLayout(editor, same)).toBe(false);
  });

  it("is offered, without a way to put it back, when it cannot be read", () => {
    expect(differsFromLayout(editor, "not json")).toBe(true);
  });
});
