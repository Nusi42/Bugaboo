import type { Schema } from "@bugaboo/api-client";
import { describe, expect, it } from "vitest";

import { newestAnswer } from "./newest-answer.ts";

/** The answer of a change of the tags of the issue at this version, with these tags. */
function answer(version: number, tags: readonly string[]): Schema<"IssueLabelsAndTagsBody"> {
  return {
    issue: "i1",
    version,
    updated_at: "2026-11-13T10:00:00Z",
    issue_type: { id: "t1", name: "Bug", icon: "bug", retired: false },
    tags: tags.map((name) => ({ id: name, name, color: "cat1" })),
    labels: [],
    may_change_tags: true,
    may_change_labels: true,
  };
}

describe("tag_01 the row of tags shows the newest answer of a change", () => {
  it("keeps the answer of a newer change when the answer of an older one comes in late", () => {
    // B's change answered first (version 7); A's (version 6) comes after it.
    const shown = newestAnswer(undefined, answer(7, ["first", "second"]));
    expect(newestAnswer(shown, answer(6, ["first"]))).toEqual(answer(7, ["first", "second"]));
  });

  it("takes the answer of a newer change", () => {
    expect(newestAnswer(answer(6, ["first"]), answer(7, []))).toEqual(answer(7, []));
  });
});
