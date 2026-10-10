import { newestDraft } from "./long-text-model.ts";
import { EditingStore, ignore } from "./long-text-store.ts";

/**
 * What the box finds out when it opens, which is not the rules of editing: a draft to offer, and
 * who is editing now (watched until they let go). `LongTextEditor` extends this.
 */
export abstract class WatchingEditor extends EditingStore {
  /**
   * Counts the endings of an editing (cancel, dispose). A call that started before one and answers
   * after it is stale: it takes nothing and starts nothing, and a lock it was given is let go.
   */
  protected generation = 0;

  /** The text in the box changed (the editor says how). */
  abstract readonly type: (text: string) => void;

  /**
   * Asks who is editing when the box opens, so a person who comes second is told before the first
   * key (UX §6). While somebody is, the question is asked again at each beat: when they let go,
   * the box opens.
   */
  protected async findHolder(): Promise<void> {
    const generation = this.generation;
    const holder = await this.deps.peek().catch(() => null);
    // Only while nothing was typed: a person who started writing meets the lock when it is taken.
    if (
      generation === this.generation &&
      holder !== null &&
      this.state.phase.kind === "open" &&
      this.state.text === this.state.saved
    ) {
      this.set({ phase: { kind: "held", lock: holder } });
      this.stopBeats ??= this.deps.startHeartbeats(this.watchHolder);
    }
  }

  protected readonly watchHolder = async (): Promise<void> => {
    if (this.state.phase.kind !== "held") {
      return;
    }
    const generation = this.generation;
    const holder = await this.deps.peek();
    if (generation !== this.generation) {
      return;
    }
    if (holder === null) {
      this.stopHeartbeats();
      this.set({ phase: { kind: "open" } });
    } else {
      this.set({ phase: { kind: "held", lock: holder } });
    }
  };

  protected async findDraft(): Promise<void> {
    const newest = newestDraft(
      [this.deps.browserDraft.read(), await this.deps.serverDraft().catch(() => null)],
      this.state.saved,
    );
    // Only while nothing was typed: a person who started writing has their own text.
    if (newest !== undefined && this.state.phase.kind === "open") {
      this.set({ offered: newest });
    }
  }

  /** Continues from the offered draft; the first key of a draft that differs takes the lock. */
  readonly continueDraft = (): void => {
    const draft = this.state.offered;
    if (draft !== null) {
      this.set({ offered: null });
      this.type(draft.text);
    }
  };

  /** Starts again from the saved text and drops the offered draft. */
  readonly dropDraft = (): void => {
    this.set({ offered: null });
    this.deps.browserDraft.forget();
    void this.deps.throwAwayServerDraft().catch(ignore);
  };
}
