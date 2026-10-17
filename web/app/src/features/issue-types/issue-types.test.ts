import { describe, expect, it } from "vitest";

import { issueTypeLineOf } from "./issue-types.ts";

const bug = {
  id: "bug",
  name: "Bug",
  icon: "bug",
  retired: false,
  version: 3,
  may_manage: true,
  created_in: { kind: "organization" },
} as const;

describe("the line of an issue type", () => {
  it("says nothing about a project until the list was asked about one", () => {
    expect(issueTypeLineOf(bug)?.here).toBeUndefined();
  });

  it("says whether the project asked about offers the type, by its own line or the one above", () => {
    const own = issueTypeLineOf({
      ...bug,
      offered_here: false,
      own_offer: false,
      may_change_offer: true,
    });
    expect(own?.here).toEqual({ offered: false, own: false, mayChange: true });
    const inherited = issueTypeLineOf({ ...bug, offered_here: true, may_change_offer: false });
    expect(inherited?.here).toEqual({ offered: true, own: undefined, mayChange: false });
  });

  it("knows where the type was created and whether the person looks after it", () => {
    const line = issueTypeLineOf(bug);
    expect(line?.createdIn).toEqual({ kind: "organization" });
    expect(line?.mayManage).toBe(true);
    expect(issueTypeLineOf({ ...bug, may_manage: false })?.mayManage).toBe(false);
  });

  it("is nothing for an item that came without what a line needs", () => {
    expect(issueTypeLineOf({ id: "bug", name: "Bug" })).toBeUndefined();
  });
});
