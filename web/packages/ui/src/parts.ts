/**
 * How a component says what it is. It never carries a style value or a class name of its own: it
 * puts `data-part` (one of `part`, from the theme file), `data-variant` and `data-state` on its
 * elements, and the one style sheet made from `design/theme.kdl` does the rest (ADR-0026). The
 * words of a variant or a state must be ones that the theme file styles for that part, so a
 * misspelled word is a type error, not a part that silently looks like nothing.
 */
import { partWords, type PartName } from "./tokens.ts";

type WordsOf<
  Name extends PartName,
  Kind extends "variant" | "state",
> = (typeof partWords)[Name][Kind][number];

/** The words of one part; a word that is not there (`false`, `undefined`) is left out. */
export interface PartChoices<Name extends PartName> {
  /** What kind of the part it is: `primary`, `wide`, … (`data-variant`). */
  readonly variants?: readonly (WordsOf<Name, "variant"> | false | undefined)[];
  /** What it is doing now: `busy`, … (`data-state`). */
  readonly states?: readonly (WordsOf<Name, "state"> | false | undefined)[];
}

/** The attributes to spread onto an element that is this part. */
export interface PartAttributes {
  readonly "data-part": string;
  readonly "data-variant"?: string;
  readonly "data-state"?: string;
}

function joined(names: readonly (string | false | undefined)[]): string | undefined {
  const present = names.filter(Boolean);
  return present.length === 0 ? undefined : present.join(" ");
}

/** `{...partOf(part.button, { variants: [look] })}`: the attributes of an element that is a part. */
export function partOf<Name extends PartName>(
  name: Name,
  choices: PartChoices<Name> = {},
): PartAttributes {
  const variant = joined(choices.variants ?? []);
  const state = joined(choices.states ?? []);
  return {
    "data-part": name,
    ...(variant === undefined ? {} : { "data-variant": variant }),
    ...(state === undefined ? {} : { "data-state": state }),
  };
}
