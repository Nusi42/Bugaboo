import { describe, expect, it } from "vitest";

import type { WordingState } from "../../shared/use-wording.ts";
import { testWording } from "../../shared/wording-testing.ts";
import { mayApply } from "./many-changes.ts";

describe("iss_09 a change to many issues is not started blind", () => {
  it("keeps Apply off until the words of the preview are there", () => {
    const states: readonly [WordingState, boolean][] = [
      [{ state: "loading" }, false],
      [{ state: "failed" }, false],
      [{ state: "ready", wording: testWording }, true],
    ];
    for (const [words, allowed] of states) {
      expect(mayApply(true, words)).toBe(allowed);
      expect(mayApply(false, words)).toBe(false);
    }
  });
});
