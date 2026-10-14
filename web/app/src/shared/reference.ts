/** A member of an object from the API, whatever it holds. */
export function memberOf(value: unknown, name: string): unknown {
  if (typeof value !== "object" || value === null) {
    return undefined;
  }
  const found: unknown = Reflect.get(value, name);
  return found;
}

/** The IDs a value of a person, people, group or issue field names, as the server showed them. */
export function referenceIds(value: unknown): readonly string[] {
  const each: readonly unknown[] = Array.isArray(value) ? value : [value];
  return each.flatMap((one) => {
    const id = memberOf(one, "id");
    return typeof id === "string" ? [id] : [];
  });
}
