import type { ReactNode } from "react";

import { partOf } from "./parts.ts";
import { part } from "./tokens.ts";

/**
 * A choice written as people read it: its number, if it has one, in the code font, and its name
 * (`4100 Product development`), the one way a choice is drawn wherever a screen can draw more than
 * plain text (TIME-13, UX §7.3). Where only text fits (a list of the browser, the name read out for a
 * screen reader) the server's `shown_as` says the same words.
 */
export function ChoiceText({
  number,
  name,
}: {
  readonly number?: string | null | undefined;
  readonly name: string;
}): ReactNode {
  if (number === undefined || number === null) {
    return name;
  }
  return (
    <>
      <span {...partOf(part.choiceNumber)}>{number}</span> {name}
    </>
  );
}
