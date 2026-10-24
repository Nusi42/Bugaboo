import { Fragment } from "react";

import { partOf } from "./parts.ts";
import { part } from "./tokens.ts";

const gaps = /(\s+)/u;

interface Piece {
  /** Where the piece starts in the text: no two pieces start at the same place, so it is the key. */
  readonly start: number;
  readonly piece: string;
  readonly isWord: boolean;
}

/** The words of a text and the gaps between them, each with where it starts. */
function piecesOf(text: string): readonly Piece[] {
  const pieces: Piece[] = [];
  let start = 0;
  for (const piece of text.split(gaps)) {
    if (piece !== "") {
      pieces.push({ start, piece, isWord: !gaps.test(piece) });
      start += piece.length;
    }
  }
  return pieces;
}

/**
 * A text whose words never break at a hyphen: a file name like *quarter-report.pdf* goes to the next
 * line as a whole, and is cut only where it is longer than a line, like an issue ID.
 */
export function UnbrokenWords({ text }: { readonly text: string }) {
  return (
    <>
      {piecesOf(text).map(({ start, piece, isWord }) =>
        isWord ? (
          <span key={start} {...partOf(part.unbrokenWord)}>
            {piece}
          </span>
        ) : (
          <Fragment key={start}>{piece}</Fragment>
        ),
      )}
    </>
  );
}

/** A sentence that has the name of a file in it: the name keeps its words together, the rest flows. */
export function SentenceWithName({
  sentence,
  name,
}: {
  readonly sentence: string;
  readonly name: string | null | undefined;
}) {
  const at = name === null || name === undefined ? -1 : sentence.indexOf(name);
  if (at < 0 || name === null || name === undefined) {
    return sentence;
  }
  return (
    <>
      {sentence.slice(0, at)}
      <UnbrokenWords text={name} />
      {sentence.slice(at + name.length)}
    </>
  );
}
