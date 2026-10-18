import { Alert, Button, buttonLook, partOf, part } from "@bugaboo/ui";

/** One of the two answers of a note about a draft. */
export interface DraftAnswer {
  readonly label: string;
  readonly onPress: () => void;
  readonly isPending?: boolean;
}

export interface DraftNoteProps {
  /** What the note says. */
  readonly text: string;
  /** *Continue* or *Put my changes back*: the answer that takes the draft up (the one filled button). */
  readonly take?: DraftAnswer | undefined;
  /** *Throw away*: the answer that lets it go. */
  readonly drop: DraftAnswer;
}

/**
 * The note about a draft that was kept: the text of the offer, the filled button that takes the
 * draft up and the plain one that throws it away. Written once for the description, a comment, a new
 * issue and the editors of layouts and workflows (ISS-17, ADR-0019).
 */
export function DraftNote({ text, take, drop }: DraftNoteProps) {
  return (
    <Alert>
      <div {...partOf(part.issueNote)}>
        <span>{text}</span>
        {take === undefined ? null : (
          <Button look={buttonLook.primary} onPress={take.onPress}>
            {take.label}
          </Button>
        )}
        <Button isPending={drop.isPending ?? false} onPress={drop.onPress}>
          {drop.label}
        </Button>
      </div>
    </Alert>
  );
}
