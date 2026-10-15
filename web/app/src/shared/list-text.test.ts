import { describe, expect, it } from "vitest";

import { detailsText, listText } from "./list-text.ts";

describe("txt_01 lists in a sentence are joined the way the language does it", () => {
  it("lists one, two and many names", () => {
    expect(listText(["Ana"])).toBe("Ana");
    expect(listText(["Ana", "Ben"])).toBe("Ana and Ben");
    expect(listText(["Ana", "Ben", "Cem"])).toBe("Ana, Ben, and Cem");
  });

  it("puts short facts side by side", () => {
    expect(detailsText(["Text", "For issues"])).toBe("Text · For issues");
  });
});
