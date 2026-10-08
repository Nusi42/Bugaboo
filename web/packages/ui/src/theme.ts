import {
  defaultDarkTheme,
  defaultLightTheme,
  pageAttributes,
  themeIds,
  type ThemeId,
} from "./tokens.ts";

/** What a person can pick: follow the device, or one of the built-in themes. */
export const themeChoices = ["device", ...themeIds] as const;
export type ThemeChoice = (typeof themeChoices)[number];

export const defaultThemeChoice: ThemeChoice = "device";

/** The media query that says the device prefers a dark look. */
export const deviceDarkQuery = "(prefers-color-scheme: dark)";

export function isThemeChoice(value: unknown): value is ThemeChoice {
  return themeChoices.some((choice) => choice === value);
}

/** The light and the dark theme that "Match my device" uses (THM-02). */
export interface DeviceThemes {
  readonly light: ThemeId;
  readonly dark: ThemeId;
}

export const defaultDeviceThemes: DeviceThemes = {
  light: defaultLightTheme,
  dark: defaultDarkTheme,
};

/** "Match my device" becomes the light theme on a light device and the dark one on a dark device. */
export function resolveTheme(
  choice: ThemeChoice,
  deviceIsDark: boolean,
  device: DeviceThemes = defaultDeviceThemes,
): ThemeId {
  if (choice === "device") {
    return deviceIsDark ? device.dark : device.light;
  }
  return choice;
}

/** Puts the theme on the page; the generated token CSS reacts to the `look` attribute of the page frame. */
export function applyTheme(theme: ThemeId): void {
  document.documentElement.setAttribute(pageAttributes.look, theme);
}
