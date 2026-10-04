import { defaultDarkTheme, defaultLightTheme, themeIds, type ThemeId } from "./tokens.ts";

/** What a person can pick: follow the device, or one of the built-in themes. */
export const themeChoices = ["device", ...themeIds] as const;
export type ThemeChoice = (typeof themeChoices)[number];

export const defaultThemeChoice: ThemeChoice = "device";

const storageKey = "bugaboo.theme";

export function isThemeChoice(value: unknown): value is ThemeChoice {
  return themeChoices.some((choice) => choice === value);
}

/** "Match my device" becomes Daylight on a light device and Dusk on a dark one. */
export function resolveTheme(choice: ThemeChoice, deviceIsDark: boolean): ThemeId {
  if (choice === "device") {
    return deviceIsDark ? defaultDarkTheme : defaultLightTheme;
  }
  return choice;
}

export interface ChoiceReader {
  readonly getItem: (key: string) => string | null;
}

export interface ChoiceWriter {
  readonly setItem: (key: string, value: string) => void;
}

/**
 * The saved choice, or the default when nothing valid is saved or storage is blocked.
 * Storage is passed as a getter because merely touching `localStorage` can throw.
 */
export function readStoredChoice(getStorage: () => ChoiceReader): ThemeChoice {
  try {
    const saved = getStorage().getItem(storageKey);
    return isThemeChoice(saved) ? saved : defaultThemeChoice;
  } catch {
    return defaultThemeChoice;
  }
}

/** Remembers the choice on this device; storage that is blocked is not an error for the person. */
export function storeChoice(getStorage: () => ChoiceWriter, choice: ThemeChoice): void {
  try {
    getStorage().setItem(storageKey, choice);
  } catch {
    // The choice then only lasts until the page is closed.
  }
}

/** Puts the theme on the page; the generated token CSS reacts to `data-look`. */
export function applyTheme(theme: ThemeId): void {
  document.documentElement.dataset["look"] = theme;
}
