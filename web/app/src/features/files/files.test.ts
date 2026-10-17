import { describe, expect, it } from "vitest";

import { done } from "./fake-uploads.ts";
import { hasPreviewComing } from "./files.ts";

describe("att_04 a preview that is on its way", () => {
  it("is waited for while any file of the list says its preview is pending", () => {
    expect(hasPreviewComing([])).toBe(false);
    expect(hasPreviewComing([{ ...done, preview: "ready" }])).toBe(false);
    expect(hasPreviewComing([{ ...done, preview: "without" }])).toBe(false);
    expect(
      hasPreviewComing([
        { ...done, preview: "ready" },
        { ...done, preview: "pending" },
      ]),
    ).toBe(true);
  });
});
