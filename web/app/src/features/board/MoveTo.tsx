import { t } from "@bugaboo/messages";
import { Button, buttonLook, part, partOf, Window, ItemActions } from "@bugaboo/ui";
import { useId, useState } from "react";

import { itemActionsWords } from "../../shared/item-actions-words.ts";
import { windowLabels } from "../../shared/window-labels.ts";
import type { BoardCard, BoardColumn } from "./board-data.ts";
import { columnId, columnName, keyOf, mayGo, moveOf, reasonOf } from "./card-moves.ts";
import { useBoardState } from "./holding.ts";

/** One column a card can be moved to, or the reason it cannot: a button, with the reason under its name. */
function MoveChoice({
  card,
  column,
  at,
  own,
  onChoose,
}: {
  readonly card: BoardCard;
  readonly column: BoardColumn;
  readonly at: number;
  readonly own: number;
  readonly onChoose: (at: number) => void;
}) {
  const { field } = useBoardState();
  const reasonId = useId();
  const move = moveOf(card, at);
  const reason =
    at === own ? t("board.here", { key: keyOf(card) }) : reasonOf(card, move, { column, field });
  const allowed = at !== own && mayGo(move);
  return (
    <li>
      <Button
        look={buttonLook.item}
        wide
        isDisabled={!allowed}
        {...(reason === undefined ? {} : { "aria-describedby": reasonId })}
        onPress={() => {
          onChoose(at);
        }}
      >
        {columnName(column)}
      </Button>
      {reason === undefined ? null : (
        <span id={reasonId} {...partOf(part.boardMoveReason)}>
          {reason}
        </span>
      )}
    </li>
  );
}

/**
 * *Move to …* on a card (UX §4.6, TEST-A11Y): the columns as choices, the ones the card cannot go to
 * with their reason. It does what a drag does: the same status change through the workflow, *Ask for*
 * and all, and *Undo*. On a phone it is the way: nothing is dragged.
 */
export function MoveTo({ card, column }: { readonly card: BoardCard; readonly column: number }) {
  const { columns, moves } = useBoardState();
  const [open, setOpen] = useState(false);
  const key = keyOf(card);
  return (
    <>
      <ItemActions
        {...itemActionsWords(key)}
        actions={[
          {
            id: "move-to",
            label: t("board.move-to"),
            onAction: () => {
              setOpen(true);
            },
          },
        ]}
      />
      <Window
        isOpen={open}
        onOpenChange={setOpen}
        title={t("board.move-to-title", { key })}
        labels={windowLabels()}
      >
        <ul aria-label={t("board.moves-list", { key })} {...partOf(part.actionMenu)}>
          {columns.map((each, at) => (
            <MoveChoice
              key={columnId(each)}
              card={card}
              column={each}
              at={at}
              own={column}
              onChoose={(chosen) => {
                setOpen(false);
                moves.move(card, chosen);
              }}
            />
          ))}
        </ul>
      </Window>
    </>
  );
}
