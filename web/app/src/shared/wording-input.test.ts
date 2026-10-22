import { describe, expect, it } from "vitest";

import { side_words } from "../../../packages/wasm/generated/bb_wasm.js";
import { testWording } from "./wording-testing.ts";

describe("ntf_02 the shared wording refuses an input it cannot read", () => {
  it("throws, instead of giving no words, for a side that Rust does not know", () => {
    expect(testWording.sideWords("tags", { shown: "names", names: [] })).toBeNull();
    const odd = JSON.stringify({ field: "tags", side: { shown: "colors" } });
    expect(() => side_words(odd)).toThrow(/unreadable input/u);
  });
});
