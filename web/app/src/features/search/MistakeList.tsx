import { part, partOf } from "@bugaboo/ui";
import type { Mistake } from "@bugaboo/wasm-search";

import { mistakeText } from "./mistake-text.ts";
import { isUnfinished } from "./unfinished.ts";

export interface MistakeListProps {
  /** The ID of the list, which the box names with `aria-describedby`. */
  readonly id: string;
  readonly mistakes: readonly Mistake[];
  /** Where the text ends while the person is still typing at it; none once they have left the box. */
  readonly unfinishedAt?: number | undefined;
}

/** What tells one mistake from the others: where it is and which it is. */
function mistakeKey(mistake: Mistake): string {
  return `${String(mistake.span.start)}:${mistake.key}`;
}

/**
 * What is wrong with the text, in plain words (UX design section 4.3). The words of a mistake are
 * the person's own text, put into the message as text: React draws them as text, so a tag or an
 * attribute in a field name is shown as typed and never runs (WP-02 review).
 */
export function MistakeList({ id, mistakes, unfinishedAt }: MistakeListProps) {
  if (mistakes.length === 0) {
    return null;
  }
  return (
    <ul {...partOf(part.searchMistakes)} id={id}>
      {mistakes.map((mistake) => (
        <li
          key={mistakeKey(mistake)}
          {...partOf(part.searchMistake, {
            variants: isUnfinished(mistake, unfinishedAt) ? ["hint"] : [],
          })}
        >
          {mistakeText(mistake)}
        </li>
      ))}
    </ul>
  );
}
