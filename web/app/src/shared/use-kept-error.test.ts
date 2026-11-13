import { describe, expect, it } from "vitest";

import { keptAfter, type Kept, type ShownRead } from "./use-kept-error.ts";

const refused = new Error("not found");
const issue = (address: string) => ["issues", "one", address];

/** The refusal shown after each of the reads, the state carried from one to the next as the hook does. */
function shownAfter(reads: readonly ShownRead[]): unknown[] {
  let kept: Kept | null = null;
  return reads.map((read) => {
    const next = keptAfter(kept, read);
    kept = next.kept;
    return next.shown;
  });
}

describe("ux_01 a refusal stays on screen while it is read again", () => {
  it("keeps the refusal while the same read is done again, and drops it when the read answers", () => {
    expect(
      shownAfter([
        { queryKey: issue("WEB-9"), data: undefined, error: refused },
        { queryKey: issue("WEB-9"), data: undefined, error: null },
        { queryKey: issue("WEB-9"), data: { id: "i9" }, error: null },
      ]),
    ).toEqual([refused, refused, null]);
  });

  it("never shows the refusal of one read for another", () => {
    expect(
      shownAfter([
        { queryKey: issue("WEB-9"), data: undefined, error: refused },
        { queryKey: issue("WEB-1"), data: undefined, error: null },
      ]),
    ).toEqual([refused, null]);
  });

  it("settles at once when a read has an answer and a refusal, and shows the refusal", () => {
    const read = { queryKey: issue("WEB-1"), data: { id: "i1" }, error: refused };
    const first = keptAfter({ key: "earlier", error: refused }, read);
    const second = keptAfter(first.kept, read);
    expect(second).toEqual(first);
    expect(first.shown).toBe(refused);
  });

  it("keeps the same state when nothing changed, so the page draws once", () => {
    const read = { queryKey: issue("WEB-9"), data: undefined, error: refused };
    const first = keptAfter(null, read);
    expect(keptAfter(first.kept, read).kept).toBe(first.kept);
  });
});
