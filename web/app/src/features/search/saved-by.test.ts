import { t } from "@bugaboo/messages";
import { describe, expect, it } from "vitest";

describe("ux_4_3 a shared saved search in the sidebar", () => {
  it("says who it is by with a dot and plain spaces, kept in one piece by the sidebar, not by hidden characters", () => {
    expect(t("search.saved-by", { person: "Ada Brandt" })).toBe("· Ada Brandt");
  });
});
