import { afterEach, describe, expect, it, vi } from "vitest";

import { isMessageKey, t } from "./index.ts";

describe("ux_07 message catalog", () => {
  it("returns plain messages", () => {
    expect(t("app.name")).toBe("Bugaboo");
  });

  it("fills in placeholders", async () => {
    await import("./generated/shell.ts");
    expect(t("shell.signed-in-as", { name: "Ana" })).toBe("Signed in as Ana");
  });
});

describe("ux_07 keys from outside", () => {
  it("knows its own keys and nothing else", () => {
    expect(isMessageKey("app.name")).toBe(true);
    expect(isMessageKey("error.nothing.here")).toBe(false);
    expect(isMessageKey("toString")).toBe(false);
  });
});

describe("perf_04 message parts", () => {
  // These run in this order: the first one needs a part that nothing has loaded yet.
  it("finds a key once its part is loaded, and fails loudly before", async () => {
    expect(isMessageKey("fields.list")).toBe(false);
    expect(() => t("fields.list")).toThrow(/not loaded/u);
    await import("./generated/fields.ts");
    expect(isMessageKey("fields.list")).toBe(true);
    expect(t("fields.list")).toBe("Fields");
  });

  it("keeps the texts of the first page apart from the texts of an area", () => {
    expect(isMessageKey("app.name")).toBe(true);
    expect(isMessageKey("roles.intro")).toBe(false);
  });

  it("has the texts that the server sends by key, and that shared code looks up, on the first page", () => {
    const keys = [
      "error.auth.not-signed-in",
      "error.request.wrong-token",
      "error.server.internal",
      "issue-field.title",
      "issue-field.description",
      "issue-field.priority",
      "issue-field.reported-by",
      "starting-role.guest",
    ];
    expect(keys.filter((key) => !isMessageKey(key))).toEqual([]);
  });

  it("holds every key in exactly one part, and finds all of them once all are loaded", async () => {
    const parts = await import("./generated/all.ts");
    const keys = Object.values(parts).flatMap((messages) => Object.keys(messages));
    expect(Object.keys(parts)).toContain("shared");
    expect(keys.length).toBeGreaterThan(1000);
    expect(new Set(keys).size).toBe(keys.length);
    expect(keys.filter((key) => !isMessageKey(key))).toEqual([]);
  });
});

/** A fresh copy of the package: the parts that other tests loaded are not in it. */
function freshPackage(): Promise<typeof import("./index.ts")> {
  vi.resetModules();
  return import("./index.ts");
}

describe("perf_04 a text that is missing in production", () => {
  afterEach(() => {
    vi.unstubAllEnvs();
    vi.restoreAllMocks();
  });

  it("reports the key once and shows nothing, so the page keeps working", async () => {
    vi.stubEnv("DEV", false);
    const report = vi.spyOn(console, "error").mockImplementation(() => {
      /* silent */
    });
    const { t: fresh } = await freshPackage();
    expect(fresh("people.search")).toBe("");
    expect(fresh("people.search")).toBe("");
    expect(report).toHaveBeenCalledTimes(1);
    expect(String(report.mock.calls[0]?.[0])).toContain("people.search");
    expect(fresh("roles.list")).toBe("");
    expect(report).toHaveBeenCalledTimes(2);
  });

  it("throws in development and under test", async () => {
    const { t: fresh } = await freshPackage();
    expect(() => fresh("people.search")).toThrow(/people\.search.*not loaded/u);
  });
});
