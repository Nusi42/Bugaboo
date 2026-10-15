/** A member of an object from the API, whatever it holds. */
export function memberOf(value: unknown, name: string): unknown {
  if (typeof value !== "object" || value === null) {
    return undefined;
  }
  const found: unknown = Reflect.get(value, name);
  return found;
}

/**
 * A person, group or issue picked in the form of a new issue: the ID the API takes and the name the
 * form shows until the issue is made (there is no server answer yet to show it by).
 */
export interface Picked {
  readonly id: string;
  readonly name: string;
}

/** Whether a value is a thing picked in the form (an ID with a name). */
export function isPicked(value: unknown): value is Picked {
  return typeof memberOf(value, "id") === "string" && typeof memberOf(value, "name") === "string";
}

/**
 * What a field of the form of a new issue holds for the API: things picked become their IDs (one
 * ID for one thing, a list for several), anything else stays as it is.
 */
export function forApi(picks: readonly Picked[] | Picked | null): string | readonly string[] | null;
export function forApi(value: unknown): unknown;
export function forApi(value: unknown): unknown {
  if (isPicked(value)) {
    return value.id;
  }
  if (Array.isArray(value) && value.length > 0 && value.every((each) => isPicked(each))) {
    return value.map((each: Picked) => each.id);
  }
  return value;
}

/** The things a value of a person, people, group or issue field names, each with its ID and name. */
export function picksOf(value: unknown): readonly Picked[] {
  const each: readonly unknown[] = Array.isArray(value) ? value : [value];
  return each.flatMap((one) => {
    const id = memberOf(one, "id");
    const name = memberOf(one, "name");
    const key = memberOf(one, "key");
    if (typeof id !== "string") {
      return [];
    }
    return [{ id, name: typeof name === "string" ? name : typeof key === "string" ? key : "" }];
  });
}
