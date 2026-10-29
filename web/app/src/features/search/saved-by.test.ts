import { t } from "@bugaboo/messages";
import { describe, expect, it } from "vitest";

describe("ux_4_3 a shared saved search in the sidebar", () => {
  it("is read as its name, a comma and who it is by, while the person shows on a second line without a dot", () => {
    expect(`Questions${t("search.saved-by-spoken", { person: "Ada Brandt" })}`).toBe(
      "Questions, Ada Brandt",
    );
  });
});
