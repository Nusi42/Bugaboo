import { readFileSync } from "node:fs";
import { join } from "node:path";

import { describe, expect, it } from "vitest";

import { colorsOf, looksOf, readMedia, tokensOf } from "./media-theme.ts";
import { repoRoot } from "./ports.ts";

// The roadmap picture of the front page (docs/media/roadmap.svg) is drawn by hand. Its colors come
// from the theme (as the generated sheet has them), and its milestones and feature counts from
// the overview table of docs/04-milestones.md; this test keeps both the same, so the picture
// cannot drift.

const roadmap = readMedia("roadmap.svg");
const { light, dark } = looksOf(roadmap);
const plan = readFileSync(join(repoRoot, "docs/04-milestones.md"), "utf8");
const cargo = readFileSync(join(repoRoot, "Cargo.toml"), "utf8");

interface Tile {
  readonly state: string;
  readonly code: string;
  readonly name: string;
  readonly features: number;
}

/** Group `index` of a match, or an empty text. */
function group(match: readonly string[], index: number): string {
  return match[index] ?? "";
}

const planRows: readonly Tile[] = [
  ...plan.matchAll(/^\| \*\*(M\d+)\*\* \| (.+?) \| .+? \| (\d+) \|$/gmu),
].map((m: readonly string[]) => ({
  state: "",
  code: group(m, 1),
  name: group(m, 2),
  features: Number(group(m, 3)),
}));
const tiles: readonly Tile[] = [
  ...roadmap.matchAll(
    /<g class="(done|next|later)" data-milestone="(M\d+)" data-features="(\d+)">\s*<title>[^:]+: (.+?), \d+ features<\/title>/gu,
  ),
].map((m: readonly string[]) => ({
  state: group(m, 1),
  code: group(m, 2),
  name: group(m, 4),
  features: Number(group(m, 3)),
}));

/** The features of some tiles added up. */
function featuresOf(some: readonly Tile[]): number {
  return some.reduce((sum, tile) => sum + tile.features, 0);
}

describe("the roadmap picture of the front page", () => {
  it("uses the colors of the default themes", () => {
    expect(colorsOf(light)).toEqual(tokensOf(light, "daylight"));
    expect(colorsOf(dark)).toEqual(tokensOf(dark, "dusk"));
    expect(colorsOf(light)).toHaveLength(8);
    expect(colorsOf(dark)).toHaveLength(8);
  });

  it("has one tile for every milestone of the plan, with its name and number of features", () => {
    expect(planRows).toHaveLength(16);
    expect(tiles.map(({ code, name, features }) => ({ code, name, features }))).toEqual(
      planRows.map(({ code, name, features }) => ({ code, name, features })),
    );
  });

  it("marks the done milestones first, then one next, and the version follows the rule", () => {
    expect(
      tiles
        .map((tile) => tile.state)
        .join(" ")
        .replace(/(done )+next (later ?)+/u, "ok"),
    ).toBe("ok");
    const upToM14 = tiles.slice(0, 15);
    const total = featuresOf(upToM14);
    const done = featuresOf(upToM14.filter((tile) => tile.state === "done"));
    expect(roadmap).toContain(`data-total="${total}" data-done="${done}"`);
    const minor = Math.floor((9 * done) / total);
    expect(cargo).toContain(`version = "0.${minor}.`);
    expect(roadmap).toContain(`Now: 0.${minor}.`);
  });

  it("stands still when a person asks for less motion", () => {
    expect(roadmap).toMatch(
      /@media \(prefers-reduced-motion: reduce\)[^}]*\{[^}]*animation: none/u,
    );
  });

  it("loads nothing from outside and runs no script", () => {
    expect(roadmap).not.toMatch(/<image|<script|href=|url\(|https?:\/\/(?!www\.w3\.org)/u);
  });
});
