import { fieldWords } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";

/** What a field is called: an own field by its name, a built-in field by the word of the catalogue. */
export function fieldLabel(field: string, name: string | null): string {
  const word = fieldWords.find((each) => each === field);
  if (name !== null) {
    return name;
  }
  return word === undefined ? field : t(`issue-field.${word}`);
}
