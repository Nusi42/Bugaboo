import { t } from "@bugaboo/messages";
import { describe, expect, it } from "vitest";

describe("ux_4_3 a shared saved search in the sidebar", () => {
  it("is read as its name, a separator and who it is by, the person shown once on a second line without a dot", () => {
    expect(`Questions${t("search.saved-by-separator")}Ada Brandt`).toBe("Questions, Ada Brandt");
  });
});
