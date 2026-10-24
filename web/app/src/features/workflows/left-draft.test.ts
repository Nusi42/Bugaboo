import type { Schema } from "@bugaboo/api-client";
import { describe, expect, it } from "vitest";

import { draftOf } from "./draft-model.ts";
import { draftOfKept, keptText } from "./left-draft.ts";

const editor: Schema<"WorkflowEditorBody"> = {
  name: "Simple",
  statuses: [
    { key: "open", name: "Open", status_group: "not-started" },
    { key: "done", name: "Done", status_group: "done" },
  ],
  changes: [
    {
      from: "open",
      to: "done",
      button: "Mark as done",
      rules: {
        who: { roles: ["r1"], groups: [], assigned: true, reported_by: false },
        only_if: [],
        must_fill: [],
        ask_for: [],
      },
    },
  ],
  roles: [],
  groups: [],
  fields: [],
};

describe("wf_02 the draft a refused publish left is kept as text", () => {
  it("comes back the way it was kept, rules and all", () => {
    const draft = draftOf(editor);
    expect(draftOfKept(keptText(draft))).toEqual(draft);
  });

  it("is not made from text that is not a draft", () => {
    for (const text of ["", "{", "[]", '{"name":1}', '{"name":"x","statuses":[1],"changes":[]}']) {
      expect(draftOfKept(text)).toBeUndefined();
    }
  });
});
