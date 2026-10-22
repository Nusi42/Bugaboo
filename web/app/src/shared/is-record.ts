/** Whether a value read from outside (storage, a browser answer) is a plain object: not `null`, not an array. */
export function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
