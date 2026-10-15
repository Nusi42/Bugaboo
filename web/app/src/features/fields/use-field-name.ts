import { fieldNameOf } from "../../shared/field-names.ts";
import { useFields } from "./fields.ts";

/**
 * What a field is called, for a sentence about it: an own field of the project by its name, any
 * other by the word the catalogue has for it.
 */
export function useFieldName(project: string | undefined, key: string): string {
  const found = useFields({ project }).data?.fields.find((each) => each.id === key);
  return fieldNameOf({ code: key, name: found?.name });
}
