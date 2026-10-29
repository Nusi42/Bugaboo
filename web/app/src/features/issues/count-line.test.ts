import { t } from "@bugaboo/messages";
import { describe, expect, it } from "vitest";

import { countLine } from "./count-line.ts";

describe("ux_4_2 the count line of the list", () => {
  it("says how many rows are shown once there are some", () => {
    expect(countLine(302, "Website", 13)).toBe(
      t("issues.count-line", { total: 302, scope: "Website", shown: 13 }),
    );
  });

  it("does not say 0 shown while the rows are still on their way", () => {
    expect(countLine(42, "Website", 0)).toBe(
      t("issues.count-line-loading", { total: 42, scope: "Website" }),
    );
    expect(countLine(42, "Website", 0)).not.toContain("0");
  });

  it("says 0 shown for a search that finds nothing", () => {
    expect(countLine(0, "Website", 0)).toBe(
      t("issues.count-line", { total: 0, scope: "Website", shown: 0 }),
    );
  });
});
