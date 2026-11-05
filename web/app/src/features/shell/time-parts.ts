import type { ReactNode } from "react";

import { createRegistry, useRegistry } from "./registry.ts";

/**
 * Where the parts of time stand: at the bottom of the sidebar of a computer, at the top of *Home* (on
 * a phone and a tablet, which have no sidebar), or in the bar of a phone.
 */
export const timePlace = { sidebar: "sidebar", home: "home", bar: "bar" } as const;
export type TimePlace = (typeof timePlace)[keyof typeof timePlace];

/** How a part of time is shown where it stands. */
export interface TimePartView {
  readonly place: TimePlace;
  /** The sidebar is narrowed to icons. */
  readonly narrow: boolean;
}

/** A part that the feature of time fills: the running timer and the work day (TIME-01, TIME-08). */
export interface TimePartHost {
  /** Names the part; unique. */
  readonly id: string;
  readonly render: (view: TimePartView) => ReactNode;
}

const parts = createRegistry<TimePartHost>();

/** Adds a part of time to the shell; returns the way to take it out again. */
export const addTimePart = parts.add;

/** The parts of time that were added, for the component that shows them. */
export function useTimeParts(): readonly TimePartHost[] {
  return useRegistry(parts);
}
