import { createContext, useContext } from "react";

/** How a field is called on screen, by its word; a word that is not known is shown as it is. */
export type FieldNameOf = (word: string) => string;

/**
 * The names of the readable fields, as the server gives them (the catalog for the fields every
 * issue has, the name the organization gave for an own field). The bar provides them, so a window
 * of the bar never guesses a name from the word of a field.
 */
export const FieldNames = createContext<FieldNameOf>((word) => word);

/** The function that gives the name of a field by its word. */
export function useFieldName(): FieldNameOf {
  return useContext(FieldNames);
}

/** A choice of an own field as a button shows it: its number, its name, and the two written together. */
export interface ChoiceShown {
  readonly number: string | null | undefined;
  readonly name: string;
  readonly shownAs: string;
}

/** The choice a word names in the field of a word (`budget`), as the person's fields have it. */
export type ChoiceOfWord = (field: string, word: string) => ChoiceShown | null;

/** The choice a word of the search names in an own field, by the word of the field; the bar provides it from the fields. */
export const ChoiceWords = createContext<ChoiceOfWord>(() => null);

/** The function that finds the choice a typed word names. */
export function useChoiceWords(): ChoiceOfWord {
  return useContext(ChoiceWords);
}
