import { useCallback, useEffect, useLayoutEffect, useState } from "react";

import {
  applyTheme,
  deviceDarkQuery,
  readStoredChoice,
  resolveTheme,
  storeChoice,
  type ThemeChoice,
} from "./theme.ts";
import type { ThemeId } from "./tokens.ts";

export interface ThemeState {
  /** What the person picked, which may be "Match my device". */
  readonly choice: ThemeChoice;
  /** The theme that is showing right now. */
  readonly resolved: ThemeId;
  readonly choose: (choice: ThemeChoice) => void;
}

/** The person's theme choice, kept on this device and applied to the whole page. */
export function useTheme(): ThemeState {
  const [choice, setChoice] = useState<ThemeChoice>(() => readStoredChoice(() => localStorage));
  const [deviceIsDark, setDeviceIsDark] = useState(() => matchMedia(deviceDarkQuery).matches);
  const resolved = resolveTheme(choice, deviceIsDark);

  useEffect(() => {
    const query = matchMedia(deviceDarkQuery);
    const onChange = (): void => {
      setDeviceIsDark(query.matches);
    };
    query.addEventListener("change", onChange);
    return () => {
      query.removeEventListener("change", onChange);
    };
  }, []);

  // A layout effect runs before the browser paints, so a changed theme never flashes.
  useLayoutEffect(() => {
    applyTheme(resolved);
  }, [resolved]);

  const choose = useCallback((next: ThemeChoice) => {
    storeChoice(() => localStorage, next);
    setChoice(next);
  }, []);

  return { choice, resolved, choose };
}
