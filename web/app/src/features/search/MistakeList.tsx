import { textByKey } from "@bugaboo/messages";
import { part, partOf } from "@bugaboo/ui";
import type { Mistake } from "@bugaboo/wasm-search";

export interface MistakeListProps {
  /** The ID of the list, which the box names with `aria-describedby`. */
  readonly id: string;
  readonly mistakes: readonly Mistake[];
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
export function MistakeList({ id, mistakes }: MistakeListProps) {
  if (mistakes.length === 0) {
    return null;
  }
  return (
    <ul {...partOf(part.searchMistakes)} id={id}>
      {mistakes.map((mistake) => (
        <li key={mistakeKey(mistake)} {...partOf(part.searchMistake)}>
          {textByKey(mistake.key, mistake.args)}
        </li>
      ))}
    </ul>
  );
}
