import { describe, expect, it } from "vitest";

import { addIssueBlock, issueBlocks, issueSlots, maskOf, type IssueDetail } from "./slots.ts";

const block = (): null => null;

/** An answer of the issue with these members, of which the person may change these fields. */
function answer(issue: Readonly<IssueDetail["issue"]>, changeable: readonly string[]): IssueDetail {
  return {
    issue,
    own_fields_html: [],
    may_change: true,
    may_delete: false,
    changeable: [...changeable],
  };
}

describe("the slots of the issue page", () => {
  it("draws the blocks of a slot by order, then by id, and replaces a block with the same id", () => {
    addIssueBlock(issueSlots.history, { id: "b", order: 2, component: block });
    addIssueBlock(issueSlots.history, { id: "a", order: 2, component: block });
    addIssueBlock(issueSlots.history, { id: "z", order: 1, component: block });
    addIssueBlock(issueSlots.history, { id: "b", order: 0, component: block });
    expect(issueBlocks(issueSlots.history).map((entry) => entry.id)).toEqual(["b", "z", "a"]);
    expect(issueBlocks(issueSlots.subIssues)).toEqual([]);
  });

  it("builds the mask from the answer", () => {
    const detail: IssueDetail = {
      issue: { title: "x" },
      own_fields_html: [],
      may_change: true,
      may_delete: false,
      changeable: ["title"],
    };
    const mask = maskOf(detail);
    expect(mask.sees("title")).toBe(true);
    expect(mask.sees("priority")).toBe(false);
    expect(mask.changes("title")).toBe(true);
    expect(mask.changes("priority")).toBe(false);
  });

  it("sees a built-in field the issue holds under its own member, a hidden one never", () => {
    // A reader who may see the estimate and not change it: the issue holds `estimated_time`.
    const reader = maskOf(answer({ title: "x", estimated_time: 90 }, ["title"]));
    expect(reader.sees("estimated-time")).toBe(true);
    expect(reader.changes("estimated-time")).toBe(false);
    // A reader without the field: the server leaves it out and it is not theirs to change.
    const without = maskOf(answer({ title: "x" }, ["title"]));
    expect(without.sees("estimated-time")).toBe(false);
    // Somebody who may change a field that has no value yet sees it.
    expect(maskOf(answer({ title: "x" }, ["estimated-time"])).sees("estimated-time")).toBe(true);
    // An own field is named by its ID, which is the member.
    expect(maskOf(answer({ title: "x", own_fields: { "a-b": 1 } }, [])).sees("a-b")).toBe(true);
  });
});
