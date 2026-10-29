import { t } from "@bugaboo/messages";
import { describe, expect, it } from "vitest";

describe("ux_4_3 a shared saved search in the sidebar", () => {
  it("keeps the dot with the name before it, so a line never starts with the dot", () => {
    const line = t("search.saved-by", { name: "Questions", person: "Ada Brandt" });
    expect(line).toBe("Questions · Ada Brandt");
    expect(line).not.toContain(" ·");
  });
});
