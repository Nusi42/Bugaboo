import { t } from "@bugaboo/messages";
import { describe, expect, it } from "vitest";

/** Words of the tests, not text for people. */
const words = { one: "QA", two: "Backend and QA" };

describe("ux_5 the groups of a person in the list of people", () => {
  it("says group for one and groups for more, from one text of the catalog", () => {
    expect(t("people.in-groups", { count: 1, groups: words.one })).toBe("In the group QA");
    expect(t("people.in-groups", { count: 2, groups: words.two })).toBe(
      "In the groups Backend and QA",
    );
  });
});
