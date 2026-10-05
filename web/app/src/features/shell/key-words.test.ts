import { describe, expect, it } from "vitest";

import { describeKeys, keyWord } from "./key-words.ts";

describe("ux_05 the words for keys", () => {
  it("says Ctrl on most computers and the sign for Cmd on a Mac", () => {
    expect(keyWord("Mod", false)).toBe("Ctrl");
    expect(keyWord("Mod", true)).toBe("⌘");
    expect(keyWord("k", false)).toBe("K");
    expect(keyWord("Escape", true)).toBe("Esc");
  });

  it("says a sequence as people say it", () => {
    expect(describeKeys("g h")).toBe("G then H");
  });
});
