import { fieldWords } from "./words.generated.ts";

/**
 * The member of an issue that holds a field: a built-in field is named by its word in the field
 * catalogue (`estimated-time`) and the issue holds it as `estimated_time`; an own field is held
 * under its ID, which is already the member.
 */
export function issueMemberOf(key: string): string {
  return (fieldWords as readonly string[]).includes(key) ? key.replaceAll("-", "_") : key;
}
