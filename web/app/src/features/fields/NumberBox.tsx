import { TextEditor } from "./TextEditor.tsx";

export interface NumberBoxProps {
  /** The name of the number, which the row shows and the box reads out. */
  readonly name: string;
  /** The number now; none while it has no value. */
  readonly value: number | undefined;
  /** Sends the number typed; `null` takes the value away. */
  readonly onCommit: (value: number | null) => void;
  /** What the server said was wrong with the number, in words: shown at the box. */
  readonly problem?: string | undefined;
}

/**
 * A box in a row for a number that saves in place when it is left (a share in a project, PLAN-15):
 * the same box and reader as the value of an own field of the kind *Number*, so a number is typed
 * and written one way everywhere, in the person's *Number format*.
 */
export function NumberBox({ name, value, onCommit, problem }: NumberBoxProps) {
  return (
    <TextEditor
      field={{ name, kind: "whole-number", rules: { required: false } }}
      value={value}
      inRow
      problem={problem}
      onCommit={(sent) => {
        onCommit(typeof sent === "number" ? sent : null);
      }}
    />
  );
}
