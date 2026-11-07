import { describe, expect, it } from "vitest";

import { outcomeText, runTitle, skipReasonText, type Run, type UndoSkip } from "./runs.ts";

const base: Run = {
  id: "r1",
  kind: "app-session",
  status: "done",
  started_at: "2026-11-07T10:00:00Z",
  person: { shown: "name", id: "p1", name: "Ada Admin" },
};

const skip = (rest: Partial<UndoSkip>): UndoSkip => ({
  field: "status",
  kind: "changed-since",
  by_you: false,
  ...rest,
});

describe("ai_38 the words of a run", () => {
  it("names a run by what it is of", () => {
    const named: Run = { ...base, app: "Claude Code" };
    const unnamed: Run = base;
    expect(runTitle(named)).toBe("Session of Claude Code");
    expect(runTitle(unnamed)).toBe("Session of an AI app");
    expect(runTitle({ ...base, kind: "attempt" })).toBe("Attempt of an agent");
    expect(runTitle({ ...base, kind: "many-changes" })).toBe("Change to many issues");
  });

  it("says why a field stays", () => {
    expect(skipReasonText(skip({}))).toBe("It was changed afterwards, so that change stays.");
    expect(skipReasonText(skip({ kind: "gone" }))).toBe("The issue is gone.");
    expect(skipReasonText(skip({ kind: "not-allowed" }))).toBe("You may not change it.");
    expect(skipReasonText(skip({ kind: "not-undoable" }))).toBe("It cannot be put back yet.");
  });

  it("says what became of a field after the undo", () => {
    expect(outcomeText(null)).toBeUndefined();
    expect(outcomeText({ kind: "went-back" })).toBe("Went back");
    expect(outcomeText({ kind: "kept", reason: "gone" })).toBe("Skipped: the issue is gone");
  });
});
