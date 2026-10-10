import { isMessageKey, t } from "@bugaboo/messages";

/** The name of a field of the catalogue by its key; the key itself for one this version does not know. */
export function fieldNameOf(code: string): string {
  const key = `issue-field.${code}`;
  return isMessageKey(key) ? t(key) : code;
}
