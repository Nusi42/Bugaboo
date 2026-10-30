import { Fragment, type ReactNode } from "react";

import { wholeNamePlaces } from "./names-in-sentence.ts";
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

interface Placed {
  readonly start: number;
  readonly text: string;
}

/** The pieces of a sentence between the places a value stands, each with where the piece ends up in the sentence. */
function placedAround(sentence: string, value: string): readonly Placed[] {
  const pieces: Placed[] = [];
  let start = 0;
  for (const at of wholeNamePlaces(sentence, value)) {
    pieces.push({ start, text: sentence.slice(start, at) });
    start = at + value.length;
  }
  pieces.push({ start, text: sentence.slice(start) });
  return pieces;
}

/**
 * A sentence that has a value in it (the name of a file, the start of a key): the value is drawn by
 * `children`, as a part of its own, every time the sentence has it as a whole name (not as a piece of a longer one), and the rest of the sentence flows
 * around it.
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
  // One element from both ways, so that a list item that lays out its children (a flex box) keeps
  // the sentence in one line of words.
  const shown = value === null || value === undefined || value === "" ? null : value;
  return (
    <span>
      {shown === null
        ? sentence
        : placedAround(sentence, shown).map(({ start, text }) => (
            <Fragment key={start}>
              {start > 0 ? children(shown) : null}
              {text}
            </Fragment>
          ))}
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

/** The places of the names in a sentence, in order, none inside another (a longer name wins where two start alike). */
function namePlaces(sentence: string, names: readonly string[]): readonly Placed[] {
  const found = names
    .flatMap((name) => wholeNamePlaces(sentence, name).map((start) => ({ start, text: name })))
    .toSorted(
      (one: Readonly<Placed>, other: Readonly<Placed>) =>
        one.start - other.start || other.text.length - one.text.length,
    );
  const kept: Placed[] = [];
  for (const place of found) {
    const previous = kept.at(-1);
    if (previous === undefined || place.start >= previous.start + previous.text.length) {
      kept.push(place);
    }
  }
  return kept;
}

/**
 * A sentence that lists names (*Niko Eriksen and Tomas Petrov*): a line breaks only between two names, never inside
 * one; each name goes to the next line as a whole, and is cut only where it is longer than a line.
 */
export function SentenceWithNames({
  sentence,
  names,
}: {
  readonly sentence: string;
  readonly names: readonly string[];
}) {
  const pieces: ReactNode[] = [];
  let at = 0;
  for (const { start, text } of namePlaces(sentence, names)) {
    pieces.push(
      sentence.slice(at, start),
      <span key={start} {...partOf(part.unbrokenWord)}>
        {text}
      </span>,
    );
    at = start + text.length;
  }
  pieces.push(sentence.slice(at));
  return <span>{pieces}</span>;
}
