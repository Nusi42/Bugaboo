import type { NameKey } from "@bugaboo/wasm";

/**
 * Whether `name` is the same name as one of `others`. Which names are the same is the rule of the
 * server (`bb_core::names`, through the module of `@bugaboo/wasm`): capitals and the spaces inside
 * a name make no difference. It is not written again here; `keyOf` is that rule.
 */
export function nameAmong(others: readonly string[], name: string, keyOf: NameKey): boolean {
  const key = keyOf(name);
  return others.some((other) => keyOf(other) === key);
}

/** The first name that stands twice in `names`, as it was written, if any. */
export function nameStandingTwice(names: readonly string[], keyOf: NameKey): string | undefined {
  return names.find((name, at) =>
    nameAmong(
      names.filter((_, other) => other !== at),
      name,
      keyOf,
    ),
  );
}
