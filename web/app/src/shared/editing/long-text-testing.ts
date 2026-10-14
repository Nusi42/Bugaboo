// Helpers for the tests of the long-text editor: a controller whose every dependency is a fake that
// writes down what it was asked, and answers the test chooses.
import { ApiError, type ErrorCode, type Heartbeat, type LockOutcome } from "@bugaboo/api-client";

import { LongTextEditor } from "./long-text.ts";
import type { HeldBySomebody, LongTextDeps } from "./long-text-model.ts";

export const ben = { shown: "name", id: "u2", name: "Ben" } as const;

export const taken = (token: string): LockOutcome => ({
  status: "taken",
  lock_token: token,
  expires_at: "2026-10-10T10:01:00Z",
  lease_seconds: 60,
  heartbeat_seconds: 15,
});

export const heldBy = (can: boolean): LockOutcome => ({
  status: "held",
  holder: ben,
  since: "2026-10-10T09:58:00Z",
  yours_elsewhere: false,
  can_edit_anyway: can,
});

export const stillHeld: Heartbeat = {
  lock: { status: "held", expires_at: "2026-10-10T10:01:00Z" },
  notices: [],
  draft_saved: true,
};

/** The error of a request the server refused with this code. */
export function refusal(code: ErrorCode): ApiError {
  return new ApiError(409, {
    type: `urn:bugaboo:error:${code}`,
    title: code,
    status: 409,
    code,
    message_key: "error.server.unavailable",
    params: {},
  });
}

/** Answers one after the other, the last one again and again. */
export function inTurn<Answer>(...answers: readonly Answer[]): () => Answer {
  let next = 0;
  return () => {
    const answer = answers[Math.min(next, answers.length - 1)];
    next += 1;
    if (answer === undefined) {
      throw new Error("no answer was given");
    }
    return answer;
  };
}

/** A save that is refused with this code the first time and goes through after that. */
export function refusedOnce(
  code: ErrorCode,
  seen: (input: Readonly<{ text: string; version: number; lockToken: string }>) => void,
): LongTextDeps["save"] {
  const refused = inTurn(true, false);
  return (input) => {
    seen(input);
    return refused() ? Promise.reject(refusal(code)) : Promise.resolve();
  };
}

/** What a lock held by Ben looks like to the box that asks who is editing. */
export const holding: HeldBySomebody = {
  holder: ben,
  since: "2026-10-10T09:58:00Z",
  yours_elsewhere: false,
  can_edit_anyway: true,
};

/** A promise whose answer the test gives later. */
export function deferred<Value>(): {
  readonly promise: Promise<Value>;
  readonly resolve: (value: Value) => void;
} {
  let resolve: (value: Value) => void = ignoreAnswer;
  const promise = new Promise<Value>((done) => {
    resolve = done;
  });
  return { promise, resolve };
}

/** The answer to nobody, until a promise gives the real one. */
function ignoreAnswer(): void {
  // Replaced at once by the resolver of the promise.
}

/** Lets the promises that were started finish. */
export function settle(): Promise<void> {
  return new Promise<void>((done) => {
    setTimeout(done, 0);
  });
}

/** What the fakes were asked, and the fakes themselves. */
export class Seen {
  readonly log: string[] = [];
  readonly kept: { text: string; base: number }[] = [];
  readonly saves: { text: string; version: number; lockToken: string }[] = [];
  readonly beats: { text: string; base: number }[] = [];
  /** Runs one heartbeat, as the timer would. */
  beat: () => Promise<void> = () => Promise.resolve();

  private note(what: string): Promise<void> {
    this.log.push(what);
    return Promise.resolve();
  }

  /** The fakes, with the answers of the happy path; `over` replaces some. */
  fakes(over: Partial<LongTextDeps>): LongTextDeps {
    return {
      take: (editor) => {
        this.log.push(`take ${editor}`);
        return Promise.resolve(taken("t1"));
      },
      peek: () => Promise.resolve(null),
      editAnyway: () => {
        this.log.push("edit-anyway");
        return Promise.resolve(taken("t2"));
      },
      askToFinish: () => this.note("ask"),
      heartbeat: (token, draft) => {
        this.beats.push({ text: draft?.text ?? "", base: draft?.base_version ?? 0 });
        this.log.push(`beat ${token}`);
        return Promise.resolve(stillHeld);
      },
      release: (token) => this.note(`release ${token}`),
      save: (input) => {
        this.saves.push(input);
        return Promise.resolve();
      },
      refresh: () => this.note("refresh"),
      serverDraft: () => Promise.resolve(null),
      throwAwayServerDraft: () => this.note("throw-away"),
      browserDraft: {
        read: () => null,
        write: (text, base) => {
          this.kept.push({ text, base });
        },
        forget: () => {
          this.log.push("forget");
        },
      },
      startHeartbeats: (run) => {
        this.beat = async () => {
          await run();
        };
        this.log.push("start-beats");
        return () => {
          this.log.push("stop-beats");
        };
      },
      newEditorId: () => "e1",
      ...over,
    };
  }
}

/** A controller over a text *Old words* at version 3, and the record of what it asked for. */
export function world(
  over: Partial<LongTextDeps> = {},
): Seen & { readonly editor: LongTextEditor } {
  const seen = new Seen();
  const editor = new LongTextEditor(seen.fakes(over), { saved: "Old words", version: 3 });
  return Object.assign(seen, { editor });
}
