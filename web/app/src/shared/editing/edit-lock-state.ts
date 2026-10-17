import { EditLockSession, type Answer, type SessionDeps } from "./edit-lock-session.ts";
import type { HeldBySomebody, LostReason } from "./long-text-model.ts";
import { Store } from "../store.ts";

/** Where the person stands with the edit lock on a thing that is edited as a whole (ADR-0019): a layout or a workflow. */
export type LockState =
  | { readonly phase: "taking" }
  | { readonly phase: "mine"; readonly token: string }
  | {
      readonly phase: "held";
      /** Who is editing, since when, and whether the person may take over. */
      readonly held: HeldBySomebody;
    }
  | { readonly phase: "lost" };

const taking: LockState = { phase: "taking" };
const lost: LockState = { phase: "lost" };

/**
 * The state of the edit lock on a layout or a workflow while its editor is open: taken when the editor opens,
 * kept with a heartbeat, let go when it closes. The hold on the lock is the shared session; this
 * is what the screen reads of it, a plain object to subscribe to, so every path is tested without
 * a browser.
 */
export class EditLockController extends Store<LockState> {
  private readonly session: EditLockSession;

  constructor(deps: SessionDeps) {
    super(taking);
    this.session = new EditLockSession(deps, {
      onRanOut: "lost",
      onLost: () => {
        this.replace(lost);
      },
    });
  }

  /** Asks for the lock; called when the editor opens and again by *Try again*. */
  readonly start = (): void => {
    this.replace(taking);
    void this.answer(this.session.take);
  };

  /** *Edit anyway*: the lock becomes the caller's. */
  readonly editAnyway = (): void => {
    void this.answer(this.session.editAnyway);
  };

  readonly askToFinish = (): void => {
    void this.session.askToFinish();
  };

  /** A save was refused for the lock's sake; the lock is let go or dropped as the reason says. */
  readonly refused = (reason: LostReason): void => {
    void this.session.refused(reason);
  };

  /** The editor is closed: the heartbeat stops and the lock is let go. */
  readonly dispose = (): void => {
    void this.session.end();
  };

  /** Takes the answer of an ask, unless another ask or the end came meanwhile. */
  private async answer(ask: () => Promise<Answer>): Promise<void> {
    try {
      const answer = await ask();
      if (answer.kind === "held") {
        this.replace({ phase: "held", held: answer.lock });
      } else if (answer.kind === "mine") {
        this.replace({ phase: "mine", token: answer.token });
      }
    } catch {
      this.replace(lost);
    }
  }
}
