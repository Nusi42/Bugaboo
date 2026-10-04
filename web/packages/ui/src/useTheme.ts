import { useCallback, useEffect, useState } from "react";

import {
  applyTheme,
  readStoredChoice,
  resolveTheme,
  storeChoice,
  type ThemeChoice,
} from "./theme.ts";

const darkQuery = "(prefers-color-scheme: dark)";

/** The person's theme choice, kept on this device and applied to the whole page. */
export function useTheme(): readonly [ThemeChoice, (choice: ThemeChoice) => void] {
  const [choice, setChoice] = useState<ThemeChoice>(() => readStoredChoice(localStorage));
  const [deviceIsDark, setDeviceIsDark] = useState(() => matchMedia(darkQuery).matches);

  useEffect(() => {
    const query = matchMedia(darkQuery);
    const onChange = (): void => {
      setDeviceIsDark(query.matches);
    };
    query.addEventListener("change", onChange);
    return () => {
      query.removeEventListener("change", onChange);
    };
  }, []);

  useEffect(() => {
    applyTheme(resolveTheme(choice, deviceIsDark));
  }, [choice, deviceIsDark]);

  const choose = useCallback((next: ThemeChoice) => {
    storeChoice(localStorage, next);
    setChoice(next);
  }, []);

  return [choice, choose] as const;
}
