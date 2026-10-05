import { useCallback, useLayoutEffect, useSyncExternalStore } from "react";

import { applyAppearance, currentSource, themeOf, type Appearance } from "./appearance.ts";
import { deviceDarkQuery, resolveTheme } from "./theme.ts";
import type { ThemeId } from "./tokens.ts";

export interface AppearanceState {
  readonly appearance: Appearance;
  /** The theme that is showing right now. */
  readonly resolved: ThemeId;
  /** The theme "Match my device" shows with the device's current setting. */
  readonly deviceTheme: ThemeId;
  /** Changes some of the choices and keeps the rest. */
  readonly change: (changes: Partial<Appearance>) => void;
}

function subscribeToSource(listener: () => void): () => void {
  return currentSource().subscribe(listener);
}

function readAppearance(): Appearance {
  return currentSource().read();
}

function subscribeToDevice(listener: () => void): () => void {
  const query = matchMedia(deviceDarkQuery);
  query.addEventListener("change", listener);
  return () => {
    query.removeEventListener("change", listener);
  };
}

function readDevice(): boolean {
  return matchMedia(deviceDarkQuery).matches;
}

/**
 * The person's choices about the look, applied to the whole page. Every screen that shows or
 * changes them calls this; it reads from the one source (`setAppearanceSource`).
 */
export function useAppearance(): AppearanceState {
  const appearance = useSyncExternalStore(subscribeToSource, readAppearance);
  const deviceIsDark = useSyncExternalStore(subscribeToDevice, readDevice);

  // A layout effect runs before the browser paints, so a changed choice never flashes.
  useLayoutEffect(() => {
    applyAppearance(appearance, deviceIsDark);
  }, [appearance, deviceIsDark]);

  const change = useCallback((changes: Partial<Appearance>) => {
    const source = currentSource();
    source.write({ ...source.read(), ...changes });
  }, []);

  return {
    appearance,
    resolved: themeOf(appearance, deviceIsDark),
    deviceTheme: resolveTheme("device", deviceIsDark, appearance),
    change,
  };
}
