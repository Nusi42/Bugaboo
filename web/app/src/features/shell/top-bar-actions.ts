import type { ReactNode } from "react";

import { createRegistry, useRegistry } from "./registry.ts";

/**
 * A button a feature puts at the end of the top bar (the *Assistant* button is the first). The feature
 * adds it while it applies and takes it out again when it does not, so the bar shows no trace of
 * what is off.
 */
export interface TopBarAction {
  /** Names the action; unique. */
  readonly id: string;
  /** What to draw: the button itself. */
  readonly render: () => ReactNode;
}

const actions = createRegistry<TopBarAction>();

/** Adds a button to the top bar; returns the way to take it out again. */
export const addTopBarAction = actions.add;

/** The buttons features added, in the order they were added. */
export function useTopBarActions(): readonly TopBarAction[] {
  return useRegistry(actions);
}
