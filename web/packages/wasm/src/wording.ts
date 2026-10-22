/**
 * The shapes the wording of `bb-core` takes and gives (`crates/bb-core/src/wording`). The Rust
 * types are the source: a side that its tests do not read the same way fails there.
 */

/** Some words: a text, words that follow each other, or a message of the catalog with its words. */
export type Said =
  | string
  | readonly Said[]
  | { readonly key: string; readonly args: Readonly<Record<string, Said>> };

/** One side of a change: what a field was, or what it became. */
export type WordedSide =
  | { readonly shown: "text"; readonly text: string }
  | { readonly shown: "names"; readonly names: readonly string[] }
  | { readonly shown: "people"; readonly people: readonly string[] }
  | {
      readonly shown: "typed";
      readonly kind: string;
      readonly value: unknown;
      /** The words of a number, a day or a moment in the region of the reader. */
      readonly worded?: string | null;
    }
  | { readonly shown: "plain"; readonly text: string };

/** A change of one field, with the words the reader sees for the people and the field. */
export interface WordedChange {
  readonly who: string;
  readonly field: string;
  readonly field_name: string;
  readonly before?: WordedSide | null;
  readonly after?: WordedSide | null;
}

/** What a notification says, with what its sentences need. */
export interface WordedNotice {
  readonly kind: string;
  readonly who: string;
  readonly comment?: boolean;
  readonly changes?: readonly WordedChange[];
  readonly notice?: string | null;
  readonly run_done?: boolean | null;
  readonly renewal?: { readonly project?: string | null } | null;
}
