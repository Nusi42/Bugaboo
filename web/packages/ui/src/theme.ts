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

/**
 * An own theme on the page (THM-04): its ID and the address of the style sheet the server makes
 * from its values, which holds custom properties only. The page never builds a sheet itself, so the
 * content security policy stays `style-src 'self'`.
 */
export interface OwnLook {
  readonly id: string;
  readonly sheet: string;
}

/** The link element of the sheet of an own theme, while one is on the page. */
let ownSheet: HTMLLinkElement | null = null;

/** Links the sheet of an own theme, or takes the link away when there is none. */
function linkOwnSheet(own: OwnLook | null): void {
  if (own === null) {
    ownSheet?.remove();
    ownSheet = null;
    return;
  }
  if (ownSheet === null) {
    ownSheet = document.createElement("link");
    ownSheet.rel = "stylesheet";
    document.head.append(ownSheet);
  }
  if (ownSheet.getAttribute("href") !== own.sheet) {
    ownSheet.href = own.sheet;
  }
}

/**
 * Puts the theme on the page: the generated token CSS reacts to the `look` attribute of the page
 * frame, and an own theme's sheet sets its values on top of it under `data-own-theme`.
 */
export function applyTheme(theme: ThemeId, own: OwnLook | null = null): void {
  const page = document.documentElement;
  page.setAttribute(pageAttributes.look, theme);
  linkOwnSheet(own);
  if (own === null) {
    page.removeAttribute(pageAttributes.ownTheme);
  } else {
    page.setAttribute(pageAttributes.ownTheme, own.id);
  }
}
