import { choiceNumberPattern } from "@bugaboo/api-client";
import { useState } from "react";

import { codeOf, messageOf } from "../../shared/errors.ts";

/** What a number of a choice looks like: the pattern of the server, a constant generated from it. */
const digitsOnly = new RegExp(choiceNumberPattern, "u");

/**
 * The number a person typed for a choice: none for an empty box, the digits as typed for digits
 * only (`0410` stays `0410`), and `undefined` for anything else (the box is then told to be fixed,
 * nothing is sent).
 */
export function choiceNumberOf(text: string): string | null | undefined {
  const typed = text.trim();
  if (typed === "") {
    return null;
  }
  return digitsOnly.test(typed) ? typed : undefined;
}

/** The box of a number of a choice: what is typed, and whether it was told to be fixed. */
export function useChoiceNumberBox(first?: string | null) {
  const [text, setText] = useState(first ?? "");
  const [invalid, setInvalid] = useState(false);
  // What was sent last: a refusal of the server is about that text, and goes when the text changes.
  const [sent, setSent] = useState<string | undefined>();
  return {
    text,
    setText,
    invalid,
    /** The number to send (none to take it away), or `undefined` when the box was told to be fixed. */
    read: (): string | null | undefined => {
      const typed = choiceNumberOf(text);
      setInvalid(typed === undefined);
      setSent(text);
      return typed;
    },
    /**
     * What a failure of the last change is about: the sentence for a number that another choice has
     * (shown at the box, while it still holds that number), or the failure itself for anything else.
     */
    split: (error: unknown): { readonly problem?: string; readonly other?: unknown } => {
      const problem = numberProblemOf(error);
      if (problem === undefined) {
        return error === null || error === undefined ? {} : { other: error };
      }
      return text === sent ? { problem } : {};
    },
    clear: () => {
      setText("");
    },
  };
}

/** The sentence of the server when it refuses a number because another choice of the field has it. */
export function numberProblemOf(error: unknown): string | undefined {
  return codeOf(error) === "choice.number-taken" ? messageOf(error) : undefined;
}
