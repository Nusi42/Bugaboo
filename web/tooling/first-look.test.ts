import { describe, expect, it } from "vitest";

import { firstLookPlaces } from "../packages/ui/src/first-look.ts";
import { builtScriptName, firstLookScript } from "./first-look.ts";

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
