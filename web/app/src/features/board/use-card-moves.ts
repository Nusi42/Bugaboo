import type { Conflict } from "@bugaboo/api-client";
import { useEffect, useState } from "react";

import {
  askAboutConflict,
  conflictAsker,
  conflictPromptOf,
  type ConflictAsker,
} from "../../shared/editing/index.ts";
import { undoStore } from "../../shared/optimistic-undo.ts";
import { messageOf } from "../../shared/errors.ts";
import { refusedOf } from "../../shared/refused.ts";
import {
  needsAsking,
  sendChange,
  type IssueValues,
  type OfferedChange,
  type OwnColumn,
} from "../issues/index.ts";
import { refreshBoards, type BoardCard, type BoardColumn } from "./board-data.ts";
import {
  changeOf,
  choiceMove,
  choiceOf,
  issueOf,
  mayGo,
  moveOf,
  statusMove,
} from "./card-moves.ts";
import type { Planned } from "./card-moves.ts";

/** A move that asks for something first: the card and the status change that needs it (*Ask for*, *Must be filled in*). */
export interface Asking {
  readonly card: BoardCard;
  readonly change: OfferedChange;
}

/** What a move is made on: the card, what it takes, and the version of the issue it is made on. */
interface Attempt {
  readonly card: BoardCard;
  readonly planned: Planned;
  readonly version: number;
}

/** Puts a card where it was: the old values on the version the move made, as *Undo* of the issue page does. */
function putBack(card: BoardCard, before: IssueValues, version: number): void {
  sendChange(issueOf(card), { values: before, version })
    .then(refreshBoards)
    .catch((error: unknown) => {
      undoStore.offer({ message: messageOf(error) });
    });
}

/**
 * Sends the move as the issue page sends a status change, on the version the card was read at, and
 * offers *Undo* ("WEB-139 moved to In progress"). Rejects with the refusal.
 */
async function send({ card, planned, version }: Attempt): Promise<void> {
  const made = await sendChange(issueOf(card), { values: planned.values, version });
  undoStore.offer({
    message: planned.said,
    undo: () => {
      putBack(card, planned.before, made ?? version + 1);
    },
  });
}

/** The words of a refusal that is no clash, shown where the offers to undo are shown. */
function tell(error: unknown): void {
  const refused = refusedOf(error);
  if ("reason" in refused) {
    undoStore.offer({ message: refused.reason });
  }
}

/**
 * A move that somebody else's change came before (UX §4.4, as on the issue page): the card stays
 * where it was while the screen asks *Keep Ben's change*, which reads the board again, or *Use
 * mine*, which makes the move again on the version the other change made.
 */
function askAbout(conflict: Conflict, again: (version: number) => void, asker: ConflictAsker) {
  void conflictPromptOf(conflict, {
    keepTheirs: () => {
      askAboutConflict(null);
      void refreshBoards();
    },
    useMine: () => {
      askAboutConflict(null);
      again(conflict.version ?? 0);
    },
  }).then(asker.ask);
}

/** What the board does with a card that is moved to a column, by a drag, by keys or by *Move to …*. */
export interface CardMoves {
  /** Moves the card to the column, by its place; a column it may not go to, or its own, does nothing. */
  readonly move: (card: BoardCard, column: number) => void;
  /** The move that asks for something first, while its window is open. */
  readonly asking: Asking | null;
  readonly closeAsking: () => void;
  /** What the window of *Ask for* sends the change with. */
  readonly ask: (values: IssueValues) => Promise<void>;
  /** The issues whose move is on its way: their cards show it. */
  readonly moving: readonly string[];
}

/** What the moves need to know about the board: its columns and the field of its columns, when it is an own one. */
export interface MovesOf {
  readonly columns: readonly BoardColumn[];
  readonly field: OwnColumn | undefined;
}

/** The planned move of a card to a column that needs nothing asked, or none for a column the card cannot go to. */
function plan(card: BoardCard, column: number, board: MovesOf): Planned | Asking | undefined {
  const move = moveOf(card, column);
  const target = board.columns[column];
  if (move === undefined || !mayGo(move) || target === undefined) {
    return undefined;
  }
  if (move.to !== null) {
    const change = changeOf(card, move);
    if (change === undefined) {
      return undefined;
    }
    return needsAsking(change) ? { card, change } : statusMove(card, change);
  }
  const choice = choiceOf(board.field, target);
  return board.field === undefined || choice === undefined
    ? undefined
    : choiceMove(card, { id: board.field.id, choice }, target);
}

/** Whether what `plan` found is a move that asks first. */
function isAsking(found: Planned | Asking): found is Asking {
  return "change" in found;
}

/** The moves on their way, and the way to make one: a clash asks over the page, any other refusal is told. */
function useRunning(asker: ConflictAsker) {
  const [moving, setMoving] = useState<readonly string[]>([]);
  const run = (attempt: Attempt): void => {
    const id = issueOf(attempt.card);
    setMoving((before) => [...before, id]);
    send(attempt)
      .catch((error: unknown) => {
        const refused = refusedOf(error);
        if ("conflict" in refused) {
          askAbout(
            refused.conflict,
            (version) => {
              run({ ...attempt, version });
            },
            asker,
          );
        } else {
          tell(error);
        }
      })
      .finally(() => {
        setMoving((before) => before.filter((each) => each !== id));
        void refreshBoards();
      });
  };
  return { moving, run };
}

/**
 * What the window of *Ask for* sends the change with: the same send as a move that asks for nothing,
 * and the same asking when somebody else's change came first (the window closes, the page asks).
 */
function askOf(
  asking: Asking | null,
  run: (attempt: Attempt) => void,
  asker: ConflictAsker,
): (values: IssueValues) => Promise<void> {
  return async (values) => {
    if (asking === null) {
      return;
    }
    const attempt = {
      card: asking.card,
      planned: statusMove(asking.card, asking.change, values),
      version: asking.card.issue.version ?? 0,
    };
    try {
      await send(attempt);
      void refreshBoards();
    } catch (error) {
      const refused = refusedOf(error);
      if (!("conflict" in refused)) {
        throw error;
      }
      askAbout(
        refused.conflict,
        (version) => {
          run({ ...attempt, version });
        },
        asker,
      );
    }
  };
}

/**
 * The moves of the cards of a board. A drop or a key makes the status change of the workflow, which
 * the issue page makes too; the server judges it again and says why not (ADR-0030), and a clash asks
 * over the page until it is answered, or until the screen goes.
 */
export function useCardMoves(board: MovesOf): CardMoves {
  const [asking, setAsking] = useState<Asking | null>(null);
  const [asker] = useState(conflictAsker);
  const { moving, run } = useRunning(asker);
  useEffect(() => {
    asker.stay();
    return asker.leave;
  }, [asker]);
  return {
    asking,
    moving,
    closeAsking: () => {
      setAsking(null);
    },
    ask: askOf(asking, run, asker),
    move: (card, column) => {
      const found = plan(card, column, board);
      if (found === undefined) {
        return;
      }
      if (isAsking(found)) {
        setAsking(found);
      } else {
        run({ card, planned: found, version: card.issue.version ?? 0 });
      }
    },
  };
}
