import type { ReactNode } from "react";

import { partOf } from "./parts.ts";
import { part } from "./tokens.ts";

export interface StatesInPlaceProps<State extends string> {
  /** The state shown now. */
  readonly current: State;
  /** Every state the part can be in. */
  readonly states: readonly State[];
  /** What a state shows. */
  readonly render: (state: State) => ReactNode;
}

/**
 * The states of a part in one place (UX §6.4): every state stands in the same cell, the ones not shown unseen and out
 * of reach of the keyboard, the pointer and screen readers, so the part is as large as the largest state from the start
 * and nothing under it moves when the state changes (the file of *Download my data*: being created, then ready).
 */
export function StatesInPlace<State extends string>({
  current,
  states,
  render,
}: StatesInPlaceProps<State>) {
  return (
    <div {...partOf(part.statesInPlace)}>
      {states.map((state) =>
        state === current ? (
          <div key={state} {...partOf(part.statesInPlaceState)}>
            {render(state)}
          </div>
        ) : (
          <div
            key={state}
            {...partOf(part.statesInPlaceState, { states: ["unseen"] })}
            aria-hidden
            inert
          >
            {render(state)}
          </div>
        ),
      )}
    </div>
  );
}
