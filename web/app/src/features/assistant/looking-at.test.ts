import { describe, expect, it } from "vitest";

import { lookingAtOf, projectOf, withoutItem } from "./looking-at.ts";

describe("ai_10 what the panel looks at", () => {
  it("looks at the issue on an issue page", () => {
    expect(lookingAtOf("/issues/WEB-142", {})).toEqual([
      { key: "issue:WEB-142", kind: "issue", value: "WEB-142" },
    ]);
  });

  it("looks at the project and the list of a project's issues", () => {
    const items = lookingAtOf("/p/WEB/issues", { s: "status group is not Done" });
    expect(items.map((each) => each.kind)).toEqual(["project", "search"]);
    expect(items[0]?.value).toBe("WEB");
    expect(items[1]?.value).toBe("status group is not Done");
  });

  it("adds the issue that is open beside the list", () => {
    const items = lookingAtOf("/p/WEB/issues", { open: "WEB-7" });
    expect(items.map((each) => each.value)).toEqual(["WEB", "WEB-7"]);
  });

  it("looks at nothing on a page that is about no work", () => {
    expect(lookingAtOf("/me/appearance", {})).toEqual([]);
  });

  it("names the project of the place for the question whether the assistant is there", () => {
    expect(projectOf("/issues/WEB-142", {})).toBe("WEB");
    expect(projectOf("/p/OPS/settings", {})).toBe("OPS");
    expect(projectOf("/search", {})).toBeUndefined();
  });

  it("leaves one item out and keeps the others", () => {
    const items = lookingAtOf("/p/WEB/issues", { open: "WEB-7" });
    expect(withoutItem(items, "project:WEB").map((each) => each.value)).toEqual(["WEB-7"]);
  });
});
