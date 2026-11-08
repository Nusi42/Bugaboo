import { useState } from "react";

import { limits, type AssistantAnswer } from "./assistant.ts";

/** One turn of the conversation, kept in the open panel only (M7-Q6). */
export type Turn =
  | { readonly id: number; readonly from: "person"; readonly text: string }
  | { readonly id: number; readonly from: "assistant"; readonly answer: AssistantAnswer };

/** What a turn says, as the next question carries it: the text the person typed, or the answer in Markdown. */
export function sentTurn(turn: Turn): { readonly from: Turn["from"]; readonly text: string } {
  return {
    from: turn.from,
    text: (turn.from === "person" ? turn.text : turn.answer.text).slice(0, limits.turn),
  };
}

/**
 * The turns a question carries: the latest ones that fit the limit of the server. The server keeps
 * none of the conversation, so every question brings what came before it.
 */
export function turnsSent(turns: readonly Turn[]): readonly ReturnType<typeof sentTurn>[] {
  return turns.slice(-limits.turns).map((turn) => sentTurn(turn));
}

/** The turns of the open panel, and the ways to add to them. */
export function useTurns(): {
  readonly turns: readonly Turn[];
  readonly addQuestion: (text: string) => void;
  readonly addAnswer: (answer: AssistantAnswer) => void;
} {
  const [turns, setTurns] = useState<readonly Turn[]>([]);
  return {
    turns,
    addQuestion: (text) => {
      setTurns((before) => [...before, { id: before.length, from: "person", text }]);
    },
    addAnswer: (answer) => {
      setTurns((before) => [...before, { id: before.length, from: "assistant", answer }]);
    },
  };
}
