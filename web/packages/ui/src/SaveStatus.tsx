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
}

/**
 * Says whether the last change has reached the server. The region is always there, so screen
 * readers announce each new state ("Saved") when it appears.
 */
export function SaveStatus({ state, labels }: SaveStatusProps) {
  return (
    <span
      {...partOf(part.saveStatus, { states: [state === "not-saved" && state] })}
      role={politeStatus}
    >
      {state === "idle" ? null : labels[state]}
    </span>
  );
}
