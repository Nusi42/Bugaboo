import { isMessageKey, t } from "@bugaboo/messages";

/** The parts of a field's key that name it, as the API sends them. */
export interface FieldKeyWords {
  readonly code: string;
  readonly message_key?: string | null | undefined;
  readonly name?: string | null | undefined;
}

/**
 * What a field is called: an own field by its name, a field of the catalogue by its message
 * (the one the server names, else the one its code makes), and the code itself for one this
 * version does not know.
 */
export function fieldNameOf({ code, message_key: messageKey, name }: FieldKeyWords): string {
  if (name !== undefined && name !== null && name !== "") {
    return name;
  }
  const key = messageKey ?? `issue-field.${code}`;
  return isMessageKey(key) ? t(key) : code;
}
