import { describe, expect, it } from "vitest";

import {
  changeWhenText,
  outcomeText,
  runActorLine,
  runTitle,
  skipReasonText,
  unlistedSkips,
  type Run,
  type UndoSkip,
} from "./runs.ts";

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
    expect(skipReasonText(skip({ by_you: true }))).toBe(
      "You changed it afterwards, so that change stays.",
    );
    expect(skipReasonText(skip({ kind: "gone" }))).toBe("The issue is gone.");
    expect(skipReasonText(skip({ kind: "not-allowed" }))).toBe("You may not change it.");
    expect(skipReasonText(skip({ kind: "not-undoable" }))).toBe(
      "It stays: this kind of change cannot be taken back yet.",
    );
  });

  it("says what became of a field after the undo", () => {
    expect(outcomeText(null)).toBeUndefined();
    expect(outcomeText({ kind: "went-back" })).toBe("Went back");
    expect(outcomeText({ kind: "not-yet" })).toBe("Not yet undone");
    expect(outcomeText({ kind: "kept", reason: "gone" })).toBe("Skipped: the issue is gone");
  });

  it("writes one time when the run began and ended in the same minute", () => {
    const run: Run = {
      ...base,
      app: "Claude Code",
      started_at: "2026-11-07T10:00:05Z",
      ended_at: "2026-11-07T10:00:40Z",
    };
    expect(runActorLine(run)).not.toContain("–");
  });

  it("writes the time alone for a change on the day of the run and the date for another day", () => {
    const run: Run = { ...base, started_at: "2026-11-07T10:00:00Z" };
    expect(changeWhenText(run, "2026-11-07T10:30:00Z")).not.toContain("2026");
    expect(changeWhenText(run, "2026-11-09T10:30:00Z")).toContain("2026");
  });
});

describe("ai_35 the skipped fields a preview counts but does not list", () => {
  it("counts the ones beyond the list", () => {
    expect(unlistedSkips({ skipped: [], skipped_count: 0 })).toBe(0);
    expect(unlistedSkips({ skipped: [{ field: "a" }, { field: "b" }], skipped_count: 7 })).toBe(5);
  });
});
