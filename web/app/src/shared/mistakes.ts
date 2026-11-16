import { t } from "@bugaboo/messages";
import { useState } from "react";

/**
 * The boxes of a form sent as a whole that hold text that is no value yet (a day or a length half
 * typed), by a name of the box. The form holds its *Save* or *Create* until none is left, and says
 * why (`MistakeNote`), so it never sends an empty value for text the person is still typing.
 */
export type Mistakes = readonly string[];

/** The mistakes with the box `box` holding no value (`wrong`), or a value again. */
export function withMistake(mistakes: Mistakes, box: string, wrong: boolean): Mistakes {
  const others = mistakes.filter((each) => each !== box);
  return wrong ? [...others, box] : others;
}

/** The mistakes of a form, and the way each box tells them. */
export interface FormMistakes {
  readonly list: Mistakes;
  /** Whether the form is held: a box holds no value yet. */
  readonly held: boolean;
  /** What the box `box` tells: its text is no value (`true`), or a value again. */
  readonly mark: (box: string) => (wrong: boolean) => void;
  /** Forgets every mistake: the form went, or another one takes its place in the same window. */
  readonly clear: () => void;
}

/** The mistakes of one form sent as a whole. */
export function useMistakes(): FormMistakes {
  const [list, setList] = useState<Mistakes>([]);
  return {
    list,
    held: list.length > 0,
    mark: (box) => (wrong) => {
      setList((before) => withMistake(before, box, wrong));
    },
    clear: () => {
      setList([]);
    },
  };
}

/** The sentence of a held form, the same on every form (and in the footer of a new issue). */
export function mistakeText(): string {
  return t("common.value-wrong");
}
