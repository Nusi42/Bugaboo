import { describe, expect, it } from "vitest";

import { EditLockSession, type SessionDeps } from "./edit-lock-session.ts";
import { doNothing } from "../do-nothing.ts";
import { rig } from "./edit-lock-testing.ts";
import { LockServer } from "./lock-server-testing.ts";
import { heldBy, settle, stillHeld, taken } from "./long-text-testing.ts";

describe("iss_17 one session of an edit lock", () => {
  it("does_not_let_go_of_a_token_that_the_latest_ask_holds", async () => {
    const { session, released, answers } = rig();
    const first = session.take();
    const second = session.take();
    // The same editor gets the same token, so the older answer carries the token of the newer.
    answers[1]?.(taken("T"));
    expect((await second).kind).toBe("mine");
    answers[0]?.(taken("T"));
    expect((await first).kind).toBe("stale");
    expect(released).toEqual([]);
    expect(session.token).toBe("T");
  });

  it("does_not_let_go_of_a_token_the_newer_ask_will_get_when_the_older_answer_comes_first", async () => {
    const { session, released, answers } = rig();
    const first = session.take();
    const second = session.take();
    answers[0]?.(taken("T"));
    await first;
    expect(released).toEqual([]);
    answers[1]?.(taken("T"));
    await second;
    expect(released).toEqual([]);
    expect(session.token).toBe("T");
  });

  it("lets_go_of_the_token_of_an_older_answer_that_is_not_the_one_held", async () => {
    const { session, released, answers } = rig();
    const first = session.take();
    const second = session.take();
    answers[1]?.(taken("T2"));
    await second;
    answers[0]?.(taken("T1"));
    await first;
    expect(released).toEqual(["T1"]);
  });
});

describe("iss_17 the end of a session and what is asked of it", () => {
  it("lets_go_of_a_lock_that_came_after_the_session_ended", async () => {
    const { session, released, answers, beating } = rig();
    const asking = session.take();
    void session.end();
    answers[0]?.(taken("T"));
    expect((await asking).kind).toBe("stale");
    expect(released).toEqual(["T"]);
    expect(beating()).toBe(0);
  });

  it("says_who_holds_the_lock_and_starts_no_heartbeat", async () => {
    const { session, answers, beating } = rig();
    const asking = session.take();
    answers[0]?.(heldBy(true));
    expect((await asking).kind).toBe("held");
    expect(beating()).toBe(0);
    expect(session.token).toBeUndefined();
  });
});

describe("iss_17 the heartbeat of a session", () => {
  it("is_lost_and_does_not_let_go_when_the_lease_ran_out_with_the_policy_lost", async () => {
    const { session, answers, beating, beat, answerWith, lost, released } = rig();
    const asking = session.take();
    answers[0]?.(taken("T"));
    await asking;
    expect(beating()).toBe(1);
    answerWith({ lock: { kind: "ran-out" }, notices: [], draft_saved: false });
    await beat();
    await settle();
    expect(lost).toEqual([["ran-out", undefined]]);
    expect(beating()).toBe(0);
    expect(released).toEqual([]);
    expect(session.token).toBeUndefined();
  });

  it("is_told_that_the_lock_was_taken_over_and_does_not_let_go_of_it", async () => {
    const { session, answers, beat, answerWith, lost, released } = rig();
    const asking = session.take();
    answers[0]?.(taken("T"));
    await asking;
    answerWith({
      lock: { kind: "taken-over", by: { shown: "name", id: "p2", name: "Ben" } },
      notices: [],
      draft_saved: false,
    });
    await beat();
    await settle();
    expect(lost).toEqual([["taken-over", "p2"]]);
    expect(released).toEqual([]);
    expect(session.token).toBeUndefined();
  });
});

describe("iss_17 an ask and the releases on their way", () => {
  it("waits_for_a_release_that_arrives_late_and_gets_the_lock_after_it", async () => {
    const server = new LockServer();
    const deps: SessionDeps = {
      take: server.take,
      editAnyway: server.take,
      askToFinish: () => Promise.resolve(),
      heartbeat: () => Promise.resolve(stillHeld),
      release: server.release,
      startHeartbeats: () => doNothing,
      newEditorId: () => "editor-1",
    };
    const session = new EditLockSession(deps, { onRanOut: "lost", onLost: doNothing });
    expect((await session.take()).kind).toBe("mine");
    // Asking again lets the lock go first; the release is still on its way.
    const again = session.take();
    await settle();
    expect(server.calls).toEqual(["take", "release T"]);
    server.arrive();
    expect((await again).kind).toBe("mine");
    // The release went through before the second ask: the lock is held, not taken away.
    expect(server.calls).toEqual(["take", "release T", "take"]);
    expect(server.held).toBe("T");
    expect(session.token).toBe("T");
  });
});
