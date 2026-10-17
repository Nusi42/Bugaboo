import { ApiError, type ErrorCode } from "@bugaboo/api-client";
import { describe, expect, it } from "vitest";

import type { LockState } from "./edit-lock-state.ts";
import { shownSaveError } from "./save-error.ts";

function refused(code: ErrorCode): ApiError {
  return new ApiError(409, {
    type: `urn:bugaboo:error:${code}`,
    title: "Refused",
    status: 409,
    code,
    message_key: "error.edit.lock-lost",
    params: {},
  });
}

const lost: LockState = { phase: "lost" };
const mine: LockState = { phase: "mine", token: "t" };

describe("cfg_01 the save bar and the note of the lost lock do not say the same thing twice", () => {
  it("leaves out a refusal for the lock while the lost note shows", () => {
    expect(shownSaveError(refused("edit.lock_lost"), lost)).toBeNull();
    expect(shownSaveError(refused("edit.taken_over"), lost)).toBeNull();
  });

  it("keeps every other refusal, and a refusal for the lock while the lock is held", () => {
    const other = refused("access.forbidden");
    expect(shownSaveError(other, lost)).toBe(other);
    // A text that changed meanwhile needs more than the lock: its own note says so.
    const changed = refused("edit.version_changed");
    expect(shownSaveError(changed, lost)).toBe(changed);
    const lockRefusal = refused("edit.lock_lost");
    expect(shownSaveError(lockRefusal, mine)).toBe(lockRefusal);
  });
});
