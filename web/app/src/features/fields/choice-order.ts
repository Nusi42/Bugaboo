import { isNumbered, type Choice } from "./fields.ts";

/** The new order after the item at `from` goes to `to`. */
function moved(ids: readonly string[], from: number, to: number): string[] {
  const next = [...ids];
  const [taken] = next.splice(from, 1);
  if (taken !== undefined) {
    next.splice(to, 0, taken);
  }
  return next;
}

/** The choices the administrator orders by hand: those without a number (a number places the others). */
export function orderable(choices: readonly Choice[]): readonly Choice[] {
  return choices.filter((choice) => !isNumbered(choice));
}

/**
 * The order of every choice after the choice at `from` among the orderable ones goes to `to`: the
 * numbered choices keep their place at the head, the rest follow in the new order, so a change
 * is always one somebody sees.
 */
export function orderAfterMove(choices: readonly Choice[], from: number, to: number): string[] {
  const head = choices.filter((choice) => isNumbered(choice)).map((choice) => choice.id);
  const rest = orderable(choices).map((choice) => choice.id);
  return [...head, ...moved(rest, from, to)];
}

/**
 * The order after the choice `dropped` was dropped before the orderable choice at `before` (the end
 * of the orderable choices when `before` is their count); none when it is not an orderable choice.
 */
export function orderAfterDrop(
  choices: readonly Choice[],
  dropped: string,
  before: number,
): string[] | undefined {
  const from = orderable(choices).findIndex((choice) => choice.id === dropped);
  return from < 0 ? undefined : orderAfterMove(choices, from, from < before ? before - 1 : before);
}

/** Where a choice stands among those put in order by hand. */
export interface Place {
  /** Its place among them, from 0. */
  readonly at: number;
  readonly first: boolean;
  readonly last: boolean;
}

/**
 * The place of a choice among the orderable ones; none for a choice with a number (a number places
 * it): it has no arrows, no handle to drag it, and no place to drop something before it.
 */
export function placeOf(choices: readonly Choice[], id: string): Place | undefined {
  const rest = orderable(choices);
  const at = rest.findIndex((choice) => choice.id === id);
  return at < 0 ? undefined : { at, first: at === 0, last: at === rest.length - 1 };
}
