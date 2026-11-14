import { partOf } from "./parts.ts";
import type { keepsRoom } from "./props.ts";
import { part } from "./tokens.ts";

/** The states of a change that have words, in the order a change goes through them. */
const wordedStates = ["saving", "saved", "not-saved"] as const;

/** The three states a change can be in, and nothing before the first change (UX §6). */
export type SaveState = "idle" | (typeof wordedStates)[number];

/** Words of the ARIA standard, not text for people. */
const politeStatus = "status";

export interface SaveStatusProps {
  readonly state: SaveState;
  /**
   * The words for the states the place can show: *Saving…*, *Saved*, *Not saved yet*. A place that
   * never shows a state leaves its words out, and keeps no room for them.
   */
  readonly labels: Readonly<Partial<Record<Exclude<SaveState, "idle">, string>>>;
  /** Whether the place keeps the room of its words ({@link keepsRoom}). */
  readonly variant?: typeof keepsRoom;
}

/**
 * Says whether the last change has reached the server. The region is always there, so screen
 * readers announce each new state ("Saved") when it appears. Every word of the place stands with it,
 * unseen and unread; a place that keeps their room is as wide as the longest of them.
 */
export function SaveStatus({ state, labels, variant }: SaveStatusProps) {
  const states = [state === "not-saved" && state];
  return (
    <span {...partOf(part.saveStatus, { states, variants: [variant] })}>
      {/* The words keep their room as drawn shapes only: no text of the page, so nothing finds
          or reads them twice. */}
      {wordedStates.map((each) =>
        labels[each] === undefined ? null : (
          <span key={each} {...partOf(part.saveStatusRoom)} data-words={labels[each]} aria-hidden />
        ),
      )}
      <span {...partOf(part.saveStatusWords)} role={politeStatus}>
        {state === "idle" ? null : labels[state]}
      </span>
    </span>
  );
}
