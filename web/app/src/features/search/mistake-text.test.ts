import { textByKey } from "@bugaboo/messages";
import type { Mistake } from "@bugaboo/wasm-search";
import { describe, expect, it } from "vitest";

import { mistakeText } from "./mistake-text.ts";

const mistake = (key: string, args: Readonly<Record<string, string>> = {}): Mistake => ({
  span: { start: 0, end: 1 },
  key,
  args,
  suggestions: [],
});

describe("the words of a mistake of a search", () => {
  it("is the text of the key, with the words for its placeholders", () => {
    const tooLong = mistake("search.mistake.word-too-long");
    expect(mistakeText(tooLong)).toBe(textByKey("search.mistake.word-too-long", {}));
    expect(mistakeText(tooLong)).toContain("too long");
    expect(mistakeText(mistake("search.mistake.prefix-too-short", { limit: "2" }))).toContain("2");
  });

  it("looks a key of another area up as it is", () => {
    expect(mistakeText(mistake("error.resource.not-found"))).toBe(
      textByKey("error.resource.not-found", {}),
    );
  });
});
