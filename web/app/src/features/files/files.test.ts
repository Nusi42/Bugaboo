import { describe, expect, it } from "vitest";

import { done } from "./fake-uploads.ts";
import {
  hasPreviewComing,
  PREVIEW_POLL_MS,
  PREVIEW_POLL_ROUNDS_MAX,
  previewPollDelay,
} from "./files.ts";

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

describe("att_04 asking for a preview a bounded number of times", () => {
  const waiting = [{ ...done, preview: "pending" as const }];

  it("asks every moment while a preview is on its way and rounds are left", () => {
    expect(previewPollDelay(waiting, 1)).toBe(PREVIEW_POLL_MS);
    expect(previewPollDelay(waiting, PREVIEW_POLL_ROUNDS_MAX - 1)).toBe(PREVIEW_POLL_MS);
  });

  it("stops after the last round, and when no preview is coming", () => {
    expect(previewPollDelay(waiting, PREVIEW_POLL_ROUNDS_MAX)).toBe(false);
    expect(previewPollDelay([{ ...done, preview: "ready" }], 1)).toBe(false);
    expect(previewPollDelay([], 0)).toBe(false);
  });
});
