import { createContext, useContext, useState } from "react";

import type { MenuAction } from "@bugaboo/ui";

import type { BoardCard, BoardColumn } from "./board-data.ts";
import { issueOf, mayGo, moveOf, reasonOf } from "./card-moves.ts";
import type { CardMoves } from "./use-card-moves.ts";

/** How a card was taken: with the pointer (a drag), or with the keys (*Space*, the arrows, *Enter*). */
export const heldBy = { pointer: "pointer", keys: "keys" } as const;
export type HeldBy = (typeof heldBy)[keyof typeof heldBy];

/** The card in hand: whose issue it is, how it was taken, the column it came from and the column the keys chose. */
export interface Held {
  readonly issue: string;
  readonly by: HeldBy;
  readonly from: number;
  readonly target: number;
}

/** What a column says about the card in hand: whether it may go there, and why not. */
export interface Offer {
  readonly faded: boolean;
  readonly reason: string | undefined;
}

/** The card in hand, and what to do with it: the one place that holds a drag and a pick-up by keys. */
export interface Holding {
  readonly held: Held | null;
  readonly card: BoardCard | undefined;
  readonly take: (card: BoardCard, from: number, by: HeldBy) => void;
  readonly release: () => void;
  /** Chooses the column a picked card would go to, by steps; the ends stay. */
  readonly steer: (step: number) => void;
  /** Drops the card on a column; the card stays in hand while the column does not take it. */
  readonly dropOn: (column: number) => void;
  /** What the column `at` says about the card in hand. */
  readonly offerOf: (at: number) => Offer;
}

/** What the card in hand needs to know about the board. */
export interface HoldingInput {
  readonly columns: readonly BoardColumn[];
  /** The name of the field the columns come from, for the words of a refusal. */
  readonly field: string;
  readonly moves: CardMoves;
  readonly cardOf: (issue: string) => BoardCard | undefined;
}

/** The state of the card in hand. */
export function useHolding({ columns, field, moves, cardOf }: HoldingInput): Holding {
  const [held, setHeld] = useState<Held | null>(null);
  const card = held === null ? undefined : cardOf(held.issue);
  const offerOf = (at: number): Offer => {
    const column = columns[at];
    if (card === undefined || column === undefined || held === null || at === held.from) {
      return { faded: false, reason: undefined };
    }
    const move = moveOf(card, at);
    return { faded: !mayGo(move), reason: reasonOf(card, move, { column, field }) };
  };
  return {
    held,
    card,
    offerOf,
    take: (taken, from, by) => {
      setHeld({ issue: issueOf(taken), by, from, target: from });
    },
    release: () => {
      setHeld(null);
    },
    steer: (step) => {
      setHeld((now) =>
        now === null
          ? now
          : { ...now, target: Math.min(columns.length - 1, Math.max(0, now.target + step)) },
      );
    },
    dropOn: (column) => {
      if (card === undefined || column === held?.from) {
        setHeld(null);
        return;
      }
      if (offerOf(column).faded) {
        return;
      }
      moves.move(card, column);
      setHeld(null);
    },
  };
}

/** What the parts of the board share: the card in hand, the moves, and the words around the columns. */
export interface BoardState {
  readonly holding: Holding;
  readonly moves: CardMoves;
  readonly columns: readonly BoardColumn[];
  /** Whether the board is a phone's: no drag, *Move to …* on every card. */
  readonly phone: boolean;
  /** The name of the field the columns come from. */
  readonly field: string;
  /** The ID of the words that say how the keys of a card work: every card is described by them. */
  readonly keysId: string;
  /** The ways a column's menu orders the cards (*Sort by Priority*): the same for every column. */
  readonly order: readonly MenuAction[];
}

export const BoardContext = createContext<BoardState | null>(null);

/** The state of the board the part stands in. */
export function useBoardState(): BoardState {
  const state = useContext(BoardContext);
  if (state === null) {
    throw new TypeError("A part of the board stands outside its board.");
  }
  return state;
}
