import { describe, expect, it } from "vitest";

import { isMessageKey, t } from "./index.ts";

describe("ux_07 message catalog", () => {
  it("returns plain messages", () => {
    expect(t("app.name")).toBe("Bugaboo");
  });

  it("fills in placeholders", () => {
    expect(t("hello.current", { theme: "Dusk" })).toBe("Your theme: Dusk.");
  });
});

describe("ux_07 keys from outside", () => {
  it("knows its own keys and nothing else", () => {
    expect(isMessageKey("app.name")).toBe(true);
    expect(isMessageKey("error.nothing.here")).toBe(false);
    expect(isMessageKey("toString")).toBe(false);
  });
});
