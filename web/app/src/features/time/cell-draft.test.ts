import { describe, expect, it } from "vitest";

import { CellDraft } from "./cell-draft.ts";

/** The minutes of a length written `2.5` or `2:30`; enough for these tests. */
function minutesOf(text: string): number {
  const [hours = "", rest] = text.split(":");
  return rest === undefined ? Number(hours) * 60 : Number(hours) * 60 + Number(rest);
}

/** A draft of a length that knows `2.5` and `2:30` are the same. */
function lengthDraft(): CellDraft {
  const draft = new CellDraft("");
  draft.compareWith((one, other) => minutesOf(one) === minutesOf(other));
  return draft;
}

/** A draft with the text `from` that the person typed `value` into and left. */
function typedAndSent(from: string, value: string) {
  const draft = new CellDraft(from);
  draft.enter();
  draft.type(value);
  draft.leave();
  return { draft, done: draft.saving(value) };
}

describe("CellDraft", () => {
  it("takes a text read when nothing was typed and the cell is not the one in use", () => {
    const draft = new CellDraft("1:00");
    expect(draft.sync("2:00")).toBe(true);
  });

  it("keeps a late older text from replacing what was typed after it", () => {
    const { draft, done } = typedAndSent("", "1:00");
    draft.enter();
    draft.type("2:00");
    expect(draft.sync("1:00")).toBe(false);
    done(true);
    draft.leave();
    expect(draft.sync("1:00")).toBe(false);
  });

  it("keeps the second of two overlapping saves when the first text is read before the second returns", () => {
    const draft = new CellDraft("");
    draft.type("1:00");
    const first = draft.saving("1:00");
    draft.type("2:00");
    const second = draft.saving("2:00");
    first(true);
    // The week read after the first save arrives while the second is on its way.
    expect(draft.sync("1:00")).toBe(false);
    second(true);
    expect(draft.syncHeld("1:00")).toBe(false);
    expect(draft.sync("2:00")).toBe(true);
  });

  it("repairs the box when the older text got in and the value sent last is read after it", () => {
    const draft = new CellDraft("");
    draft.type("2:00");
    const done = draft.saving("2:00");
    done(true);
    expect(draft.sync("1:00")).toBe(true);
    expect(draft.sync("2:00")).toBe(true);
  });
});

describe("CellDraft and the server's way of writing", () => {
  it("takes the text the server writes in its own way after one save", () => {
    const draft = new CellDraft("");
    draft.type("1.5");
    const done = draft.saving("1.5");
    expect(draft.sync("1:30")).toBe(false);
    done(true);
    expect(draft.syncHeld("1:30")).toBe(true);
  });

  it("takes the server's way of writing a value after the save came back", () => {
    const { draft, done } = typedAndSent("", "1.5");
    done(true);
    expect(draft.sync("1:30")).toBe(true);
  });
});

describe("CellDraft when saves of one cell overlapped (M6-F180)", () => {
  it("takes the server's 2:30 for the typed 2.5 when saves overlapped, as it means what was sent last", () => {
    const draft = lengthDraft();
    draft.type("1:00");
    const first = draft.saving("1:00");
    draft.type("2.5");
    const second = draft.saving("2.5");
    first(true);
    second(true);
    // The week read after the last save was drawn before that save answered.
    expect(draft.syncHeld("2:30")).toBe(true);
  });

  it("still drops a held text of an earlier save when saves overlapped", () => {
    const draft = lengthDraft();
    draft.type("1.5");
    const first = draft.saving("1.5");
    draft.type("2.5");
    const second = draft.saving("2.5");
    first(true);
    second(true);
    expect(draft.syncHeld("1:30")).toBe(false);
    expect(draft.sync("2:30")).toBe(true);
  });

  it("without a way to compare, keeps the typed 2.5 when saves overlapped", () => {
    const draft = new CellDraft("");
    draft.type("1:00");
    const first = draft.saving("1:00");
    draft.type("2.5");
    const second = draft.saving("2.5");
    first(true);
    second(true);
    expect(draft.syncHeld("2:30")).toBe(false);
    expect(draft.sync("2:30")).toBe(true);
  });
});

describe("CellDraft after a refusal, giving up, and focus", () => {
  it("leaves the draft where it was after a refused save", () => {
    const { draft, done } = typedAndSent("0:30", "x");
    done(false);
    expect(draft.sync("0:30")).toBe(false);
    expect(draft.sync("0:45")).toBe(false);
  });

  it("goes back to the saved text when given up", () => {
    const draft = new CellDraft("0:30");
    draft.type("9:99");
    draft.giveUp("0:30");
    expect(draft.sync("0:45")).toBe(true);
  });

  it("does not take another text into a cell that is in use", () => {
    const draft = new CellDraft("0:30");
    draft.enter();
    draft.type("0:40");
    expect(draft.sync("0:45")).toBe(false);
    draft.leave();
    expect(draft.sync("0:45")).toBe(false);
  });

  it("does not write the server's way of a value into the cell the person is typing in", () => {
    const draft = lengthDraft();
    draft.type("1.5");
    const done = draft.saving("1.5");
    done(true);
    draft.enter();
    expect(draft.sync("1:30")).toBe(false);
  });

  it("does not take another text into a focused cell that was not touched", () => {
    const draft = new CellDraft("0:30");
    draft.enter();
    expect(draft.sync("0:45")).toBe(false);
    draft.leave();
    expect(draft.sync("0:45")).toBe(true);
  });
});
