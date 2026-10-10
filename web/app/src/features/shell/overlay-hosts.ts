import type { ReactNode } from "react";

import { createRegistry, useRegistry } from "./registry.ts";

/**
 * A layer that a feature owns and the shell shows: the new-issue dialog is the first. The feature
 * adds it once when the app starts; the shell draws it with its other layers, so it opens over any
 * page with the same focus rules (`openOverlay`).
 */
export interface OverlayHost {
  /** Names the host; unique. */
  readonly id: string;
  /** What to draw: the layer itself, which decides by `useOverlay` whether it is open. */
  readonly render: () => ReactNode;
}

const hosts = createRegistry<OverlayHost>();

/** Adds a layer to the shell; returns the way to take it out again. */
export const addOverlayHost = hosts.add;

/** The layers features added. */
export function useOverlayHosts(): readonly OverlayHost[] {
  return useRegistry(hosts);
}
