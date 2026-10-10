import { describe, expect, it } from "vitest";

import { addIssueBlock, issueBlocks, issueSlots, maskOf, type IssueDetail } from "./slots.ts";

const block = (): null => null;

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
});
