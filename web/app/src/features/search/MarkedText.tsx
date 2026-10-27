import type { Mistake, Search } from "@bugaboo/wasm-search";
import { part, partOf } from "@bugaboo/ui";
import type { ReactNode } from "react";

import { piecesOf, type MarkKind, type Piece } from "./marks.ts";

/** A zero-width character after the text, so a text that ends in a new line keeps the room of its last line. */
const roomForTheLastLine = "​";

export interface MarkedTextProps {
  readonly text: string;
  readonly search: Search | null;
  readonly mistakes: readonly Mistake[];
}

/** One piece of the text inside the marks that apply to it, the outermost first. */
function marked(piece: Piece): ReactNode {
  return piece.marks.reduceRight<ReactNode>(
    (inside: ReactNode, kind: MarkKind) => (
      <span {...partOf(part.searchMark, { variants: [kind] })}>{inside}</span>
    ),
    piece.text,
  );
}

/**
 * The text of Advanced search again, under the box, with the fields and values marked and the
 * mistakes underlined (UX design section 4.3). The words are drawn as text, never as HTML: the
 * text is the person's own. The layer is for the eyes only; the box has the words for everybody.
 */
export function MarkedText({ text, search, mistakes }: MarkedTextProps) {
  return (
    <div {...partOf(part.searchTextMarks)} aria-hidden>
      {piecesOf(text, search, mistakes).map((piece) => (
        <span key={piece.start}>{marked(piece)}</span>
      ))}
      {roomForTheLastLine}
    </div>
  );
}
