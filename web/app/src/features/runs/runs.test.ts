import { describe, expect, it } from "vitest";

import { runTitle, skipReasonText, type Run, type UndoSkip } from "./runs.ts";

const base = { id: "r1", status: "done", started_at: "2026-11-07T10:00:00Z" } as const;

const skip = (rest: Partial<UndoSkip>): UndoSkip => ({
  field: "status",
  kind: "changed-since",
  by_you: false,
  ...rest,
});

describe("ai_38 the words of a run", () => {
  it("names a run by what it is of", () => {
    const named: Run = { ...base, kind: "app-session", app: "Claude Code" };
    const unnamed: Run = { ...base, kind: "app-session" };
    expect(runTitle(named)).toBe("Session of Claude Code");
    expect(runTitle(unnamed)).toBe("Session of an AI app");
    expect(runTitle({ ...base, kind: "attempt" })).toBe("Attempt of an agent");
    expect(runTitle({ ...base, kind: "many-changes" })).toBe("Change to many issues");
  });

  it("says why a field stays", () => {
    expect(skipReasonText(skip({}))).toBe("was changed after the run");
    expect(skipReasonText(skip({ by_you: true }))).toBe("was changed by you after the run");
    expect(skipReasonText(skip({ kind: "gone" }))).toBe("is gone");
    expect(skipReasonText(skip({ kind: "not-allowed" }))).toBe("you may not change it");
    expect(skipReasonText(skip({ kind: "not-undoable" }))).toBe("cannot be put back yet");
  });
});
