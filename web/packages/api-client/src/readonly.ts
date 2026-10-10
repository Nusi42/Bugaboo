import type { SafeHtml } from "./safe-html.ts";
import type { components } from "./schema.ts";

/**
 * A type with every property and every list read-only, all the way down. Answers of the server
 * are never edited in place, and a screen that takes one as a parameter must say so.
 */
export type ReadonlyDeep<T> = T extends SafeHtml
  ? T
  : T extends (infer Item)[]
    ? readonly ReadonlyDeep<Item>[]
    : T extends object
      ? { readonly [Key in keyof T]: ReadonlyDeep<T[Key]> }
      : T;

/** A type of the API description (`components.schemas`), read-only, by its name. */
export type Schema<Name extends keyof components["schemas"]> = ReadonlyDeep<
  components["schemas"][Name]
>;
