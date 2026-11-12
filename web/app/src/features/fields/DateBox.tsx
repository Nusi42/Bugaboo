import { MomentEditor } from "./MomentEditor.tsx";

export interface DateBoxProps {
  /** The name of the date, which the row shows and the box reads out. */
  readonly name: string;
  /** The day now as the calendar writes it (`2026-11-14`); none while it has no value. */
  readonly day: string | undefined;
  /** Sends the day typed; `null` takes the value away. */
  readonly onCommit: (day: string | null) => void;
  /**
   * The box stands in a row of the Details, which shows the name (the default). Without it the box
   * has a visible name and sends every change at once, as a box of a form does.
   */
  readonly inRow?: boolean;
  /** What the server said was wrong with the day, in words: shown at the box. */
  readonly problem?: string | undefined;
}

/**
 * The box of a date of the issue itself (*Start date*, *Due date*, PLAN-01): the same box, reader and
 * words as the value of an own field of the kind *Date*, so a day is typed and written one way
 * everywhere.
 */
export function DateBox({ name, day, onCommit, inRow = true, problem }: DateBoxProps) {
  return (
    <MomentEditor
      field={{ name, kind: "date", rules: { required: false } }}
      value={day}
      inRow={inRow}
      live={!inRow}
      problem={problem}
      onCommit={(sent) => {
        onCommit(typeof sent === "string" ? sent : null);
      }}
    />
  );
}
