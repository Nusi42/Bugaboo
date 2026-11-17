import { resolve } from "node:path";

import { build } from "vite";
import { describe, expect, it } from "vitest";

import { firstLookPlaces } from "../packages/ui/src/first-look.ts";
import { builtScriptName, firstLookScript } from "./first-look.ts";

/** How long a build of the app may take in the test. */
const BUILD_TIMEOUT_MS = 60_000;

describe("ux_01 the script of the first look in the built page", () => {
  it("is the function of the app called with the places of the app, and nothing else", () => {
    const script = firstLookScript();
    expect(script.startsWith("(function replayFirstLook(")).toBe(true);
    expect(script.trimEnd().endsWith(`)(${JSON.stringify(firstLookPlaces)});`)).toBe(true);
  });

  it("has a name that changes with its text", () => {
    expect(builtScriptName("one")).toMatch(/^assets\/first-look-[0-9a-f]{8}\.js$/u);
    expect(builtScriptName("one")).not.toBe(builtScriptName("two"));
  });
});

/** The page as the build makes it, without writing anything. */
async function builtPage(): Promise<string> {
  const output = await build({
    configFile: resolve(import.meta.dirname, "vite.config.ts"),
    logLevel: "silent",
    build: { write: false },
  });
  const bundles = Array.isArray(output) ? output : [output];
  for (const bundle of bundles) {
    if ("output" in bundle) {
      for (const file of bundle.output) {
        if (file.fileName === "index.html" && file.type === "asset") {
          return String(file.source);
        }
      }
    }
  }
  throw new Error("The build made no index.html.");
}

describe("ux_01 the place of the script of the first look in the built page", () => {
  it(
    "stands after the style sheet of the app, so the sheet of an own theme it links comes after it",
    async () => {
      const page = await builtPage();
      const sheet = page.search(/<link rel="stylesheet"[^>]*href="\/assets\/index-/u);
      const script = page.search(/<script src="\/assets\/first-look-[0-9a-f]+\.js"><\/script>/u);
      expect(sheet).toBeGreaterThan(-1);
      expect(script).toBeGreaterThan(sheet);
      // Still in the head: it runs before anything of the body is drawn.
      expect(script).toBeLessThan(page.indexOf("</head>"));
    },
    BUILD_TIMEOUT_MS,
  );
});
