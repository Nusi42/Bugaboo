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

/** The page the choices are put on: the attributes the generated CSS reacts to. */
export interface PageAttributes {
  readonly setAttribute: (name: string, value: string) => void;
  readonly removeAttribute: (name: string) => void;
}

/** The ID of the link of an own theme's sheet that the script of the first look puts in the head. */
export const firstLookSheetId = "first-look-sheet";

/** The link of the sheet the first look put in the head, if any: the app takes it over. */
function firstLookSheet(): HTMLLinkElement | null {
  const link = document.querySelector(`link#${firstLookSheetId}`);
  return link instanceof HTMLLinkElement ? link : null;
}

/** The link element of the sheet of an own theme, while one is on the page. */
let ownSheet: HTMLLinkElement | null = null;

/** Links the sheet of an own theme, or takes the link away when there is none. */
function linkOwnSheet(own: OwnLook | null): void {
  ownSheet ??= firstLookSheet();
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
  markTheme(document.documentElement, theme, own);
  linkOwnSheet(own);
}

/** Puts the attributes of a theme and of an own theme on a page frame (the page itself, or a record of it). */
export function markTheme(page: PageAttributes, theme: ThemeId, own: OwnLook | null): void {
  page.setAttribute(pageAttributes.look, theme);
  if (own === null) {
    page.removeAttribute(pageAttributes.ownTheme);
  } else {
    page.setAttribute(pageAttributes.ownTheme, own.id);
  }
}

/**
 * Links a style sheet the server made for a part of the page, not for the page as a whole: the
 * sheet of a saved own theme for its samples in the theme pickers, or the sheet of the theme
 * editor's draft for its preview (`data-preview-look` with `data-own-theme` or `data-own-draft`).
 * `loaded` is called once the browser has the sheet. Gives the function that takes the link away
 * again. The sheet holds custom properties under those attributes only, so it changes nothing else.
 */
export function linkPreviewSheet(href: string, loaded?: () => void): () => void {
  const link = document.createElement("link");
  link.rel = "stylesheet";
  if (loaded !== undefined) {
    link.addEventListener("load", loaded, { once: true });
  }
  link.href = href;
  document.head.append(link);
  return () => {
    link.remove();
  };
}
