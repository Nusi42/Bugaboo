import { Fragment, type ReactNode } from "react";

import { partOf } from "./parts.ts";
import { part } from "./tokens.ts";
import { isGap, wordsAndGaps } from "./words.ts";

interface Piece {
  /** Where the piece starts in the text: no two pieces start at the same place, so it is the key. */
  readonly start: number;
  readonly piece: string;
  readonly isWord: boolean;
}

/** The words of a text and the gaps between them, each with where it starts. */
function piecesOf(text: string): readonly Piece[] {
  let start = 0;
  return wordsAndGaps(text).map((piece) => {
    const placed = { start, piece, isWord: !isGap(piece) };
    start += piece.length;
    return placed;
  });
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

/**
 * A sentence that has a value in it (the name of a file, the start of a key): the value is drawn by
 * `children`, as a part of its own, and the rest of the sentence flows around it.
 */
export function SentenceWithValue({
  sentence,
  value,
  children,
}: {
  readonly sentence: string;
  readonly value: string | null | undefined;
  readonly children: (value: string) => ReactNode;
}) {
  const at = value === null || value === undefined ? -1 : sentence.indexOf(value);
  if (at < 0 || value === null || value === undefined) {
    return sentence;
  }
  // One element, so that a list item that lays out its children (a flex box) keeps it in one line of words.
  return (
    <span>
      {sentence.slice(0, at)}
      {children(value)}
      {sentence.slice(at + value.length)}
    </span>
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
  return (
    <SentenceWithValue sentence={sentence} value={name}>
      {(value) => <UnbrokenWords text={value} />}
    </SentenceWithValue>
  );
}
