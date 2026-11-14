import { partOf } from "./parts.ts";
import { part } from "./tokens.ts";

/** The three states a change can be in, and nothing before the first change (UX §6). */
export type SaveState = "idle" | "saving" | "saved" | "not-saved";

/** Words of the ARIA standard, not text for people. */
const politeStatus = "status";

export interface SaveStatusProps {
  readonly state: SaveState;
  /** The words for the three states: *Saving…*, *Saved*, *Not saved yet*. */
  readonly labels: Readonly<Record<Exclude<SaveState, "idle">, string>>;
  /**
   * Keeps the room of the longest of the words from the start, in any language and font: all of
   * them stand in one place and only the one of the state is seen, so nothing beside the words
   * changes its width when they come or go (UX §6.4).
   */
  readonly keepsRoom?: boolean;
}

/** The states that have words, each of which keeps its room in one place. */
const worded = ["saving", "saved", "not-saved"] as const;

/** The words of a state, or nothing before the first change. */
function wordsOf(state: SaveState, labels: SaveStatusProps["labels"]) {
  return state === "idle" ? null : labels[state];
}

/**
 * Says whether the last change has reached the server. The region is always there, so screen
 * readers announce each new state ("Saved") when it appears. With `keepsRoom` the region stands in
 * one place with every word of it, unseen and unread, which give the place its width.
 */
export function SaveStatus({ state, labels, keepsRoom = false }: SaveStatusProps) {
  const states = { states: [state === "not-saved" && state] };
  if (!keepsRoom) {
    return (
      <span {...partOf(part.saveStatus, states)} role={politeStatus}>
        {wordsOf(state, labels)}
      </span>
    );
  }
  return (
    <span {...partOf(part.saveStatus, { ...states, variants: ["keeps-room"] })}>
      {/* The words keep their room as drawn shapes only: no text of the page, so nothing finds
          or reads them twice. */}
      {worded.map((each) => (
        <span key={each} {...partOf(part.saveStatusRoom)} data-words={labels[each]} aria-hidden />
      ))}
      <span {...partOf(part.saveStatusWords)} role={politeStatus}>
        {wordsOf(state, labels)}
      </span>
    </span>
  );
}
