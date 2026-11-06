import type { KeyboardEvent } from "react";
import { Input, TextField as AriaTextField } from "react-aria-components";

import { GRID_COLUMN } from "./grid-column.ts";
import { partOf } from "./parts.ts";
import { part } from "./tokens.ts";

export interface CellInputProps {
  /** What the cell is for people who cannot see its row and column: *Start, Thursday 1*. */
  readonly label: string;
  readonly value: string;
  readonly onChange: (value: string) => void;
  /** Called when the focus comes into the cell. */
  readonly onEnter?: () => void;
  /** Called when the focus leaves the cell. */
  readonly onLeave: () => void;
  /** Called for a key pressed in the cell; the grid moves with *Enter* and gives up with *Escape*. */
  readonly onKey: (event: KeyboardEvent<HTMLInputElement>) => void;
  /** The column of the grid it stands in, so a key can go to the next row of that column. */
  readonly column: number;
  readonly readOnly?: boolean;
  /** The value cannot be read: the sentence under it says why. */
  readonly invalid?: boolean;
  /** The change is on its way to the server. */
  readonly saving?: boolean;
  /** A rule of the day was broken, which the figure shows in the warning color. */
  readonly warns?: boolean;
  /** The ID of the sentence that says what is wrong. */
  readonly describedBy?: string | undefined;
}

/**
 * One cell of a grid that is typed into (the timesheet): a box in the code font with its value at
 * the end, a red edge for a value that cannot be read and a warning edge for a broken rule. It
 * keeps no state: the grid decides what is typed, saved and said.
 */
export function CellInput(props: CellInputProps) {
  const { label, value, onChange, onEnter, onLeave, onKey, column, describedBy } = props;
  const { readOnly = false, invalid = false, saving = false, warns = false } = props;
  return (
    <AriaTextField
      aria-label={label}
      value={value}
      onChange={onChange}
      isReadOnly={readOnly}
      isInvalid={invalid}
    >
      <Input
        {...partOf(part.sheetInput, {
          states: [
            invalid && "invalid",
            saving && "saving",
            warns && "warning",
            readOnly && "readonly",
          ],
        })}
        {...{ [GRID_COLUMN]: column }}
        aria-describedby={describedBy}
        {...(onEnter === undefined ? {} : { onFocus: onEnter })}
        onBlur={onLeave}
        onKeyDown={onKey}
      />
    </AriaTextField>
  );
}
