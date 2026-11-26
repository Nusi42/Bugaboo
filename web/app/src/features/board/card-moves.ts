import { t } from "@bugaboo/messages";

import { known } from "../../shared/present.ts";
import type { IssueValues, OfferedChange, OwnColumn } from "../issues/index.ts";
import type { BoardCard, BoardColumn, CardMove } from "./board-data.ts";

/** The Issue ID of a card, as it is written in sentences. */
export function keyOf(card: BoardCard): string {
  return known(card.issue.readable_id) ?? "";
}

/** The ID of the issue behind a card. */
export function issueOf(card: BoardCard): string {
  return known(card.issue.id) ?? "";
}

/** What a column is called: a status or a choice by its name, a status group by its words, the rest by what it lacks. */
export function columnName(column: BoardColumn): string {
  if (column.status_group !== null) {
    return t(`status-group.${column.status_group}`);
  }
  return column.key ?? t("board.no-value");
}

/** What tells a column from the others: a status or a choice has its name, a status group its word, the issues without a value neither. */
export function columnId(column: BoardColumn): string {
  return `${column.status_group ?? "-"}:${column.key ?? ""}`;
}

/** Where a card may go in a column, as the server judged it; none for the column it is in. */
export function moveOf(card: BoardCard, column: number): CardMove | undefined {
  return card.moves.find((move) => move.column === column);
}

/** Whether the server says the card may go to the column. */
export function mayGo(move: CardMove | undefined): boolean {
  return move !== undefined && move.refused === null;
}

/** What names the field of the columns in the words of a refusal: the name of its field, as the person reads it. */
export interface ReasonWords {
  readonly column: BoardColumn;
  /** The name of the field the columns come from (*Status*, *Severity*). */
  readonly field: string;
}

/** Why a card may not go to a column, in words the server's reason (its code) is turned into; none where it may. */
export function reasonOf(
  card: BoardCard,
  move: CardMove | undefined,
  words: ReasonWords,
): string | undefined {
  const key = keyOf(card);
  if (move === undefined) {
    return t("board.reason-no-status-read", { key });
  }
  const refused = move.refused;
  const column = columnName(words.column);
  const reasons = {
    "no-such-status": t("board.reason-no-status", { key, column }),
    "no-such-change": t("board.reason-no-change", {
      key,
      from: card.issue.status?.name ?? "",
      column,
    }),
    "cannot-change": t("board.reason-cannot-change", { key, field: words.field }),
  } as const;
  return refused === null ? undefined : reasons[refused];
}

/** The ID of the choice of a column of a choice field: `null` for the column of issues without one, `undefined` for a choice this screen cannot tell. */
export function choiceOf(
  field: OwnColumn | undefined,
  column: BoardColumn,
): string | null | undefined {
  if (column.key === null) {
    return null;
  }
  const choices = field?.choices ?? {};
  return Object.keys(choices).find((id) => choices[id] === column.key);
}

/** The status change of the workflow of a card that the server says enters the column. */
export function changeOf(card: BoardCard, move: CardMove): OfferedChange | undefined {
  return (card.issue.status_changes ?? []).find((change) => change.to.key === move.to);
}

/** What a move is made of: the values to send, the values that take it back, and what *Undo* says. */
export interface Planned {
  readonly values: IssueValues;
  readonly before: IssueValues;
  readonly said: string;
}

/** The move of a card by a status change: its status, and *Undo* puts the old one back. */
export function statusMove(card: BoardCard, change: OfferedChange, values?: IssueValues): Planned {
  const status = card.issue.status;
  return {
    values: values ?? { status: change.to },
    before: status === undefined || status === null ? {} : { status },
    said: t("board.moved", { key: keyOf(card), column: change.to.name }),
  };
}

/** The move of a card by a choice: the field takes the choice of the column, and *Undo* puts the old value back. */
export function choiceMove(
  card: BoardCard,
  field: { readonly id: string; readonly choice: string | null },
  column: BoardColumn,
): Planned {
  const before = card.issue.own_fields?.[field.id] ?? null;
  return {
    values: { own: { [field.id]: field.choice } },
    before: { own: { [field.id]: before } },
    said: t("board.moved", { key: keyOf(card), column: columnName(column) }),
  };
}
