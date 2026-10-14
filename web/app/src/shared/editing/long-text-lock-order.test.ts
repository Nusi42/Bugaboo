import { describe, expect, it } from "vitest";

import { LockServer } from "./lock-server-testing.ts";
import { refusal, settle, world } from "./long-text-testing.ts";

// The server gives the same editor the same token again, so a release that is still on its way
// would take away the lock that was asked for after it. The editor asks only when the release is
// through (ISS-17, ADR-0019).

/** A world over the stand-in server: a save needs the lock; the first `refusals` saves are stale. */
function overServer(refusals: number) {
  const server = new LockServer();
  const texts: string[] = [];
  let refused = refusals;
  const w = world({
    take: server.take,
    release: server.release,
    save: ({ text, lockToken }) => {
      texts.push(text);
      if (refused > 0) {
        refused -= 1;
        return Promise.reject(refusal("edit.version_changed"));
      }
      return server.held === lockToken
        ? Promise.resolve()
        : Promise.reject(refusal("edit.lock_lost"));
    },
  });
  return { server, texts, w };
}

describe("iss_17 a release in flight and a new lock", () => {
  it("compare_and_save_takes_the_lock_after_the_release_and_the_save_goes_through", async () => {
    const { server, texts, w } = overServer(1);
    w.editor.open();
    w.editor.type("My words");
    await settle();
    await w.editor.save();
    expect(w.editor.snapshot().phase.kind).toBe("lost");
    const resolving = w.editor.resolve("Merged words");
    await settle();
    // The release has not reached the server: the lock is not asked for yet.
    expect(server.calls).toEqual(["take", "release T"]);
    server.arrive();
    await resolving;
    await settle();
    expect(server.calls).toEqual(["take", "release T", "take"]);
    expect(w.editor.snapshot().problem).toBeNull();
    expect(w.editor.snapshot().phase.kind).toBe("reading");
    expect(texts).toEqual(["My words", "Merged words"]);
  });

  it("cancel_then_reopen_and_type_at_once_asks_after_the_release_of_the_first_lock", async () => {
    const { server, texts, w } = overServer(0);
    w.editor.open();
    w.editor.type("First try");
    await settle();
    w.editor.cancel();
    w.editor.open();
    w.editor.type("Second try");
    await settle();
    expect(server.calls).toEqual(["take", "release T"]);
    server.arrive();
    await settle();
    expect(server.calls).toEqual(["take", "release T", "take"]);
    expect(w.editor.snapshot().phase.kind).toBe("locked");
    await w.editor.save();
    expect(w.editor.snapshot().problem).toBeNull();
    expect(texts).toEqual(["Second try"]);
  });
});
