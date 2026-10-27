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
