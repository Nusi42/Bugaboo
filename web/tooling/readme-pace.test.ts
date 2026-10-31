import { describe, expect, it } from "vitest";

import { HERO, PACE } from "../app/e2e/support/readme-pace.ts";

// The recordings of the front page are watched by people who have never seen the app: every state
// they should notice stays long enough to be read, and nothing jumps.

/** The least and the most a state that is read stays on the screen. */
const READ = { least: 1500, most: 3000 } as const;

describe("PACE", () => {
  it("holds every state to be read for one and a half to three seconds", () => {
    for (const hold of [PACE.startMs, PACE.lookMs, PACE.resultMs]) {
      expect(hold).toBeGreaterThanOrEqual(READ.least);
      expect(hold).toBeLessThanOrEqual(READ.most);
    }
  });

  it("ends on a pause longer than any other before the recording starts again", () => {
    expect(PACE.endMs).toBeGreaterThan(READ.most);
    expect(PACE.endMs).toBeGreaterThan(PACE.resultMs);
  });

  it("moves the pointer in about a second, slow enough to follow", () => {
    const move = PACE.moveSteps * PACE.stepMs;
    expect(move).toBeGreaterThanOrEqual(900);
    expect(move).toBeLessThanOrEqual(1500);
    expect(PACE.pointedMs).toBeGreaterThanOrEqual(300);
  });

  it("types at a readable pace, a few letters a second", () => {
    expect(PACE.letterMs).toBeGreaterThanOrEqual(150);
    expect(PACE.letterMs).toBeLessThanOrEqual(400);
  });
});

describe("HERO", () => {
  it("holds each theme for two and a half to three seconds", () => {
    expect(HERO.holdMs).toBeGreaterThanOrEqual(2500);
    expect(HERO.holdMs).toBeLessThanOrEqual(3000);
  });

  it("wipes to the next theme calmly, in about a second", () => {
    const wipe = HERO.steps * HERO.stepMs;
    expect(wipe).toBeGreaterThanOrEqual(800);
    expect(wipe).toBeLessThanOrEqual(1500);
  });
});
