import { useState } from "react";

import { messageOf } from "../../shared/errors.ts";
import type { BoardOptions } from "../../shared/list-view.ts";
import type { BoardColumn } from "./board-data.ts";
import { limitProblem, limitText, limitsOf } from "./card-limits.ts";

/** What stands for *Status* among the choices of the columns: the options keep `null` for it. */
export const statusChoice = "status";
/** What stands for *No rows*. */
export const noRows = "-";

/** What the boxes of *Customize* are made from. */
export interface CustomizeInput {
  readonly options: BoardOptions;
  /** The columns the board has now: the card limits are set for them. */
  readonly columns: readonly BoardColumn[];
  /** Keeps the options with the saved search; rejects with the refusal. */
  readonly onSave: (options: BoardOptions) => Promise<void>;
  readonly onClose: () => void;
}

/** The text of a box of a column: boxes are named by the key of their column, `""` for the column without a value. */
function boxOf(column: BoardColumn): string {
  return column.key ?? "";
}

/**
 * What *Customize* keeps while it is open: the field of the columns, the field of the rows, a box for
 * the card limit of each column, and what the save is doing. The limits belong to the columns the
 * board has: with another field of columns they are dropped, and the boxes wait.
 */
export function useCustomize({ options, columns, onSave, onClose }: CustomizeInput) {
  const [by, setBy] = useState(options.columns ?? statusChoice);
  const [rows, setRows] = useState(options.rows ?? noRows);
  const [texts, setTexts] = useState<Readonly<Record<string, string>>>(
    Object.fromEntries(
      columns.map((column) => [boxOf(column), limitText(options.card_limits ?? [], column.key)]),
    ),
  );
  const [pending, setPending] = useState(false);
  const [refused, setRefused] = useState<string | undefined>();
  const same = by === (options.columns ?? statusChoice);
  const problems = columns.map((column) => limitProblem(texts[boxOf(column)] ?? ""));
  return {
    by,
    setBy,
    rows,
    setRows,
    same,
    problems,
    pending,
    refused,
    textOf: (column: BoardColumn): string => texts[boxOf(column)] ?? "",
    setText: (column: BoardColumn, value: string): void => {
      setTexts((before) => ({ ...before, [boxOf(column)]: value }));
    },
    send: (): void => {
      if (same && problems.some((problem) => problem !== undefined)) {
        return;
      }
      setPending(true);
      setRefused(undefined);
      onSave({
        columns: by === statusChoice ? null : by,
        rows: rows === noRows ? null : rows,
        card_limits: same
          ? limitsOf(
              columns.map((column) => column.key),
              texts,
            )
          : [],
      })
        .then(onClose)
        .catch((error: unknown) => {
          setRefused(messageOf(error));
          setPending(false);
        });
    },
  };
}
