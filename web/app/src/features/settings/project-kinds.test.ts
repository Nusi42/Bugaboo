import { describe, expect, it } from "vitest";

import { kindsOf, otherThanKinds, projectKindKeys } from "./project-kinds.ts";

interface Keyed {
  readonly key: string;
}

const keyed = (keys: readonly string[]): readonly Keyed[] => keys.map((key) => ({ key }));
const keysOf = (settings: readonly Keyed[]): readonly string[] =>
  settings.map((setting) => setting.key);

describe("m7_wp15 the switches of the three kinds of AI in a project", () => {
  it("names agents first, AI apps next; the assistant has its own switch beside them", () => {
    expect(projectKindKeys).toEqual(["agents.enabled", "ai-apps.enabled"]);
  });

  it("takes the kinds out of the settings of a project in that order, whatever the order of the server", () => {
    const all = keyed(["issues.id-mode", "ai-apps.enabled", "agents.enabled", "issues.id-length"]);
    expect(keysOf(kindsOf(all))).toEqual(["agents.enabled", "ai-apps.enabled"]);
    expect(keysOf(otherThanKinds(all))).toEqual(["issues.id-mode", "issues.id-length"]);
  });

  it("draws nothing for a kind the server does not offer", () => {
    expect(kindsOf(keyed(["issues.id-mode"]))).toEqual([]);
  });
});
