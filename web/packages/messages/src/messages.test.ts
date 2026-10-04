import { describe, expect, it } from "vitest";

import { t } from "./index.ts";

describe("ux_07 message catalog", () => {
  it("returns plain messages", () => {
    expect(t("app.name")).toBe("Bugaboo");
  });

  it("fills in placeholders", () => {
    expect(t("hello.current", { theme: "Dusk" })).toBe("Your theme: Dusk.");
  });
});
