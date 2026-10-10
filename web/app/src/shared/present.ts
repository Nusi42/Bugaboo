/**
 * A value of an answer that is there, or nothing. The server leaves a field out when the reader may
 * not see it and sends `null` for one that is empty; the screens treat both as "not there".
 */
export function known<Value>(value: Value | null | undefined): Value | undefined {
  return value ?? undefined;
}
