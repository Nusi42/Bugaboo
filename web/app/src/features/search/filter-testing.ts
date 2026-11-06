// Helpers for the tests of what a filter button says.
import type { ChoiceOfWord, ChoiceShown } from "./field-name-context.ts";

/** Finds the choices of the list by their name, in whatever field; a word that names none is no choice. */
export function choicesNamed(...list: readonly ChoiceShown[]): ChoiceOfWord {
  return (_field, word) => list.find((each) => each.name === word) ?? null;
}
