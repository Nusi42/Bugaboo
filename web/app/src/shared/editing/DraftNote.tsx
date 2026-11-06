import { AnswerNote, type NoteAnswer } from "@bugaboo/ui";

export interface DraftNoteProps {
  /** What the note says. */
  readonly text: string;
  /** *Continue* or *Put my changes back*: the answer that takes the draft up (the one filled button). */
  readonly take?: NoteAnswer | undefined;
  /** *Throw away*: the answer that lets it go. */
  readonly drop: NoteAnswer;
}

/**
 * The note about a draft that was kept: the text of the offer, the filled button that takes the
 * draft up and the plain one that throws it away. Written once for the description, a comment, a new
 * issue and the editors of layouts and workflows (ISS-17, ADR-0019).
 */
export function DraftNote({ text, take, drop }: DraftNoteProps) {
  return <AnswerNote text={text} main={take} other={drop} />;
}
