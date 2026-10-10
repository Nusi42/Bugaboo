import { describe, expect, it } from "vitest";

import type { OfferedDraft } from "./long-text-model.ts";
import { settle, world } from "./long-text-testing.ts";

const draft = (text: string, savedAt: string): OfferedDraft => ({ text, savedAt, baseVersion: 3 });

/** The browser keeps nothing in these tests: they look at what the box shows. */
const keepsNothing = {
  read: () => null,
  write: () => {
    // Nothing is kept.
  },
  forget: () => {
    // Nothing is kept, so there is nothing to forget.
  },
};

describe("iss_11 a draft is offered when the box opens", () => {
  it("offers the newer of the browser's and the server's draft, and continues from it", async () => {
    const w = world({
      serverDraft: () => Promise.resolve(draft("From the server", "2026-10-09T10:00:00Z")),
      browserDraft: {
        ...keepsNothing,
        read: () => draft("From the browser", "2026-10-09T12:00:00Z"),
      },
    });
    w.editor.open();
    await settle();
    expect(w.editor.snapshot().offered?.text).toBe("From the browser");
    w.editor.continueDraft();
    await settle();
    expect(w.editor.snapshot().text).toBe("From the browser");
    expect(w.editor.snapshot().phase.kind).toBe("locked");
  });

  it("offers nothing that equals the saved text", async () => {
    const w = world({
      serverDraft: () => Promise.resolve(draft("Old words", "2026-10-09T10:00:00Z")),
    });
    w.editor.open();
    await settle();
    expect(w.editor.snapshot().offered).toBeNull();
  });

  it("drops the offered draft on both sides when the person starts again", async () => {
    const w = world({
      serverDraft: () => Promise.resolve(draft("Yesterday", "2026-10-09T10:00:00Z")),
    });
    w.editor.open();
    await settle();
    w.editor.dropDraft();
    await settle();
    expect(w.editor.snapshot().offered).toBeNull();
    expect(w.log).toEqual(expect.arrayContaining(["forget", "throw-away"]));
  });
});
