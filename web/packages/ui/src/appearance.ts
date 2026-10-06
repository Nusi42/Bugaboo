import {
  applyTheme,
  defaultThemeChoice,
  deviceDarkQuery,
  isThemeChoice,
  resolveTheme,
  type ThemeChoice,
} from "./theme.ts";
import { darkThemeIds, lightThemeIds, variants, type ThemeId } from "./tokens.ts";

export const spacingChoices = ["comfortable", "compact"] as const;
export type SpacingChoice = (typeof spacingChoices)[number];

export type LightThemeId = (typeof lightThemeIds)[number];
export type DarkThemeId = (typeof darkThemeIds)[number];

/** Everything a person chooses about how Bugaboo looks (CFG-04, THM-01, THM-02, THM-06). */
export interface Appearance {
  /** The theme, or "Match my device". */
  readonly theme: ThemeChoice;
  /** The light theme that "Match my device" uses on a light device. */
  readonly light: LightThemeId;
  /** The dark theme that "Match my device" uses on a dark device. */
  readonly dark: DarkThemeId;
  readonly spacing: SpacingChoice;
  /** *Less motion*, also when the device does not ask for it. */
  readonly lessMotion: boolean;
  /** *Larger text*. */
  readonly largerText: boolean;
}

export const defaultAppearance: Appearance = {
  theme: defaultThemeChoice,
  light: "daylight",
  dark: "dusk",
  spacing: "comfortable",
  lessMotion: false,
  largerText: false,
};

function isLight(value: unknown): value is LightThemeId {
  return lightThemeIds.some((id) => id === value);
}

function isDark(value: unknown): value is DarkThemeId {
  return darkThemeIds.some((id) => id === value);
}

function isSpacing(value: unknown): value is SpacingChoice {
  return spacingChoices.some((choice) => choice === value);
}

/**
 * The appearance in a saved text. Anything that is missing or not valid is the default for that
 * one part, so a choice that no longer exists never breaks the page.
 */
export function parseAppearance(text: string | null): Appearance {
  let saved: unknown = null;
  try {
    saved = text === null ? null : JSON.parse(text);
  } catch {
    saved = null;
  }
  const read = (key: string): unknown =>
    typeof saved === "object" && saved !== null ? Reflect.get(saved, key) : undefined;
  const theme = read("theme");
  const light = read("light");
  const dark = read("dark");
  const spacing = read("spacing");
  const lessMotion = read("lessMotion");
  const largerText = read("largerText");
  return {
    theme: isThemeChoice(theme) ? theme : defaultAppearance.theme,
    light: isLight(light) ? light : defaultAppearance.light,
    dark: isDark(dark) ? dark : defaultAppearance.dark,
    spacing: isSpacing(spacing) ? spacing : defaultAppearance.spacing,
    lessMotion: typeof lessMotion === "boolean" ? lessMotion : defaultAppearance.lessMotion,
    largerText: typeof largerText === "boolean" ? largerText : defaultAppearance.largerText,
  };
}

/** The theme that shows for this appearance on a device that is light or dark. */
export function themeOf(appearance: Appearance, deviceIsDark: boolean): ThemeId {
  return resolveTheme(appearance.theme, deviceIsDark, {
    light: appearance.light,
    dark: appearance.dark,
  });
}

/** The page the choices are put on: the attributes the generated CSS reacts to. */
export interface PageAttributes {
  readonly setAttribute: (name: string, value: string) => void;
  readonly removeAttribute: (name: string) => void;
}

interface Choice {
  readonly attribute: string;
  readonly value: string;
}

function switchOn(page: PageAttributes, on: boolean, choice: Choice): void {
  if (on) {
    page.setAttribute(choice.attribute, choice.value);
  } else {
    page.removeAttribute(choice.attribute);
  }
}

/** Puts spacing, text size and motion on the page (the theme goes with `applyTheme`). */
export function applyChoices(appearance: Appearance, page: PageAttributes): void {
  switchOn(page, appearance.spacing === "compact", variants.compact);
  switchOn(page, appearance.largerText, variants["larger-text"]);
  switchOn(page, appearance.lessMotion, variants["less-motion"]);
}

/** Everything of the appearance, on the page. */
export function applyAppearance(appearance: Appearance, deviceIsDark: boolean): void {
  applyTheme(themeOf(appearance, deviceIsDark));
  applyChoices(appearance, document.documentElement);
}

/**
 * Where the appearance comes from and goes to. The app reads it from one source only, so the
 * place can change (the browser now, the server with *My settings*) without touching a screen.
 */
export interface AppearanceSource {
  readonly read: () => Appearance;
  readonly write: (next: Appearance) => void;
  /** Calls `listener` after every change; returns the way to stop. */
  readonly subscribe: (listener: () => void) => () => void;
}

/** The storage of the browser, as far as the appearance needs it. */
export interface ChoiceStore {
  readonly getItem: (key: string) => string | null;
  readonly setItem: (key: string, value: string) => void;
}

const storageKey = "bugaboo.appearance";

/**
 * The appearance kept on this device. Storage is passed as a getter because merely touching
 * `localStorage` can throw (blocked site data); then the choices last until the page is closed.
 */
export function storageSource(getStore: () => ChoiceStore): AppearanceSource {
  const listeners = new Set<() => void>();
  let current: Appearance | null = null;
  return {
    read: () => {
      if (current === null) {
        try {
          current = parseAppearance(getStore().getItem(storageKey));
        } catch {
          current = defaultAppearance;
        }
      }
      return current;
    },
    write: (next) => {
      current = next;
      try {
        getStore().setItem(storageKey, JSON.stringify(next));
      } catch {
        // The choice then only lasts until the page is closed.
      }
      for (const listener of listeners) {
        listener();
      }
    },
    subscribe: (listener) => {
      listeners.add(listener);
      return () => {
        listeners.delete(listener);
      };
    },
  };
}

/** Removes the choices this device kept (the ones of the pages before sign-in). */
export function forgetStoredAppearance(): void {
  try {
    localStorage.removeItem(storageKey);
  } catch {
    // Nothing was kept that could be removed.
  }
}

let source: AppearanceSource = storageSource(() => localStorage);

/** WP-07 calls this with a source that keeps the choices with the person's other settings. */
export function setAppearanceSource(next: AppearanceSource): void {
  source = next;
}

/** The source the app reads the appearance from. */
export function currentSource(): AppearanceSource {
  return source;
}

/**
 * Applies the saved appearance right away. Call it before the first render, so the page never
 * shows the wrong theme or spacing for a moment.
 */
export function applyInitialAppearance(): void {
  applyAppearance(source.read(), matchMedia(deviceDarkQuery).matches);
}
