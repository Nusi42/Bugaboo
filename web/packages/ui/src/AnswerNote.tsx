import type { ReactNode } from "react";

import { Alert } from "./Alert.tsx";
import { Button } from "./Button.tsx";
import { partOf } from "./parts.ts";
import { alertTone, buttonLook, type AlertTone } from "./props.ts";
import { part } from "./tokens.ts";

/** An answer of a note: a button. */
export interface NoteAnswer {
  readonly label: string;
  readonly onPress: () => void;
  readonly isPending?: boolean;
}

export interface AnswerNoteProps {
  /** What the note says: words, with values in the code face if it has any. */
  readonly text: ReactNode;
  /** What the note is (a hint by default), as for an `Alert`. */
  readonly tone?: AlertTone;
  /** The answer the screen is for: the one filled button. A note may leave it out. */
  readonly main?: NoteAnswer | undefined;
  /** The other answer: a plain button. A note may leave it out. */
  readonly other?: NoteAnswer | undefined;
}

/**
 * A note that asks a question and brings its two answers with it (UX §6.1: the way on is always a
 * button inside the note). The main answer is filled, the other one plain.
 */
export function AnswerNote({ text, tone = alertTone.info, main, other }: AnswerNoteProps) {
  return (
    <Alert tone={tone}>
      <div {...partOf(part.answerNote)}>
        <span>{text}</span>
        {main === undefined ? null : (
          <Button
            look={buttonLook.primary}
            isPending={main.isPending ?? false}
            onPress={main.onPress}
          >
            {main.label}
          </Button>
        )}
        {other === undefined ? null : (
          <Button isPending={other.isPending ?? false} onPress={other.onPress}>
            {other.label}
          </Button>
        )}
      </div>
    </Alert>
  );
}
