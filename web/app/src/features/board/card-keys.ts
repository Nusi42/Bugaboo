import { part } from "@bugaboo/ui";

import { heldBy, type BoardState } from "./holding.ts";
import type { BoardCard } from "./board-data.ts";
import { issueOf } from "./card-moves.ts";

/** What the card of a column is told of the keys: the card, the column it stands in, and the board it stands on. */
export interface CardKeyInput {
  readonly card: BoardCard;
  readonly column: number;
  readonly state: BoardState;
  /** The element of the card: where the arrows start. */
  readonly from: HTMLElement;
}

const cardSelector = `[data-part="${part.boardCard}"]`;
const cellSelector = `[data-part="${part.boardCell}"]`;

/** Puts the focus on the title of a card, which is where the keys of a card are read. */
function focusCard(card: HTMLElement | undefined): void {
  card?.querySelector<HTMLElement>("a")?.focus();
}

/** The card above or below in the same column (`step` of -1 or 1). */
function cardBeside(from: HTMLElement, step: number): HTMLElement | undefined {
  const cards = [...(from.parentElement?.querySelectorAll<HTMLElement>(cardSelector) ?? [])];
  return cards[cards.indexOf(from) + step];
}

/** The first card of the next column of the same row that has one, to the start or to the end. */
function cardInNextColumn(from: HTMLElement, step: number): HTMLElement | undefined {
  let column =
    from.closest(cellSelector)?.[step < 0 ? "previousElementSibling" : "nextElementSibling"];
  while (column !== null && column !== undefined) {
    const found = column.querySelector<HTMLElement>(cardSelector);
    if (found !== null) {
      return found;
    }
    column = column[step < 0 ? "previousElementSibling" : "nextElementSibling"];
  }
  return undefined;
}

/** The step a key of arrow left or right is: to the start or to the end of the reading direction. */
function sideStep(key: string): number {
  return key === "ArrowLeft" ? -1 : 1;
}

/** The keys of a card in hand: the arrows choose a column, *Enter* or *Space* drops it, *Esc* puts it down. */
function keysOfHeld(event: KeyboardEvent, { state }: CardKeyInput): boolean {
  const { holding } = state;
  if (event.key === "ArrowLeft" || event.key === "ArrowRight") {
    holding.steer(sideStep(event.key));
  } else if (event.key === "Enter" || event.key === " ") {
    holding.dropOn(holding.held?.target ?? 0);
  } else if (event.key === "Escape") {
    holding.release();
  } else {
    return false;
  }
  return true;
}

/** The keys of a card that is not in hand: *Space* picks it up, the arrows move the focus between cards and columns. */
function keysOfFree(event: KeyboardEvent, { card, column, state, from }: CardKeyInput): boolean {
  if (event.key === " ") {
    state.holding.take(card, column, heldBy.keys);
    return true;
  }
  const next = {
    ArrowUp: () => cardBeside(from, -1),
    ArrowDown: () => cardBeside(from, 1),
    ArrowLeft: () => cardInNextColumn(from, -1),
    ArrowRight: () => cardInNextColumn(from, 1),
  }[event.key];
  if (next === undefined) {
    return false;
  }
  focusCard(next());
  return true;
}

/**
 * The keys of a card (TEST-A11Y, UX §4.6): *Space* picks it up, the arrows choose a column, *Enter*
 * drops it and *Esc* cancels; when it is not in hand the arrows move the focus between the cards and
 * the columns. A button of the card (*Move to …*) keeps its own keys.
 */
export function cardKeys(event: KeyboardEvent, input: CardKeyInput): void {
  if (event.target instanceof HTMLButtonElement || event.altKey || event.ctrlKey || event.metaKey) {
    return;
  }
  const { held } = input.state.holding;
  const mine = held?.by === heldBy.keys && held.issue === issueOf(input.card);
  if (mine ? keysOfHeld(event, input) : keysOfFree(event, input)) {
    event.preventDefault();
  }
}
