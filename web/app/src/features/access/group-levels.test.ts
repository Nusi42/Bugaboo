import { describe, expect, it } from "vitest";

import { groupLevels } from "./group-levels.ts";

const because = ["The role Team member gives Can change for the whole organization."];

describe("sec_03 the same reason is read once", () => {
  it("puts fields with the same level and reason in one group, the biggest first", () => {
    const groups = groupLevels([
      { name: "Reported by", level: "can-see", sentences: ["Because of Guest."] },
      { name: "Title", level: "can-change", sentences: because },
      { name: "Status", level: "can-change", sentences: because },
    ]);
    expect(groups.map((group) => group.names)).toEqual([["Title", "Status"], ["Reported by"]]);
    expect(groups[0]?.level).toBe("can-change");
  });

  it("keeps apart the same level for another reason", () => {
    const groups = groupLevels([
      { name: "Title", level: "can-see", sentences: ["A."] },
      { name: "Status", level: "can-see", sentences: ["B."] },
    ]);
    expect(groups).toHaveLength(2);
  });

  it("is one group when every entry has the same level for the same reason", () => {
    const groups = groupLevels([
      { name: "Title", level: "hidden", sentences: [] },
      { name: "Status", level: "hidden", sentences: [] },
    ]);
    expect(groups).toHaveLength(1);
    expect(groups[0]?.names).toEqual(["Title", "Status"]);
  });
});
