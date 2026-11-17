import { ownSheetPattern } from "./own-sheet.ts";
import { storageKey } from "./storage.ts";
import { deviceDarkQuery, firstLookSheetId } from "./theme.ts";
import { pageAttributes, variants } from "./tokens.ts";

// The look of the first frame, before the app runs: the page frame carries the person's theme from
// its first paint, so a dark theme never flashes light while the app loads (UX §6.4). The app keeps
// what it put on the page frame for a light and for a dark device; a small script in the head of
// the page, which runs before anything is drawn, puts the one for the device back. The script is
// `replayFirstLook`, written into the built page by `web/tooling/first-look.ts`.

/** What the page frame carries for one look of the device: its attributes, and the sheet of an own theme. */
export interface FramePage {
  readonly attributes: Readonly<Record<string, string>>;
  readonly sheet: string | null;
}

/** What the page frame carries on a light and on a dark device. */
export interface FirstLook {
  readonly light: FramePage;
  readonly dark: FramePage;
}

/** What the script in the head needs to know, from the one place each is defined. */
export interface FirstLookPlaces {
  /** Where the app keeps the first look in the storage of the browser. */
  readonly key: string;
  /** The media query that says the device prefers a dark look. */
  readonly darkQuery: string;
  /** The only attributes the script may put on the page frame. */
  readonly attributes: readonly string[];
  /** The ID of the link of the sheet of an own theme, which the app takes over. */
  readonly sheetId: string;
  /** The pattern of the server's sheets of own themes (`ownSheetPattern`): any other address is never linked. */
  readonly sheetPattern: string;
}

export const firstLookPlaces: FirstLookPlaces = {
  key: storageKey.firstLook,
  darkQuery: deviceDarkQuery,
  attributes: [
    pageAttributes.look,
    pageAttributes.ownTheme,
    ...Object.values(variants).map((variant) => variant.attribute),
  ],
  sheetId: firstLookSheetId,
  sheetPattern: ownSheetPattern,
};

/**
 * Puts the first look the app kept back on the page frame. It runs in the head of the page as a
 * script of its own, before the app and before anything is drawn, so it uses nothing from outside
 * this function: its text is the script. Anything kept that is not as expected changes nothing; the
 * app puts the look on the page as soon as it runs.
 */
export function replayFirstLook(places: FirstLookPlaces): void {
  try {
    const kept: unknown = JSON.parse(localStorage.getItem(places.key) ?? "null");
    const side = matchMedia(places.darkQuery).matches ? "dark" : "light";
    const shown: unknown =
      typeof kept === "object" && kept !== null ? Reflect.get(kept, side) : undefined;
    if (typeof shown !== "object" || shown === null) {
      return;
    }
    const attributes: unknown = Reflect.get(shown, "attributes");
    const frame = document.documentElement;
    for (const name of places.attributes) {
      const value: unknown =
        typeof attributes === "object" && attributes !== null
          ? Reflect.get(attributes, name)
          : undefined;
      if (typeof value === "string") {
        frame.setAttribute(name, value);
      }
    }
    const sheet: unknown = Reflect.get(shown, "sheet");
    if (typeof sheet === "string" && new RegExp(places.sheetPattern, "u").test(sheet)) {
      // The page waits for the sheet before it draws, so an own theme shows from the first frame too.
      const link = document.createElement("link");
      link.id = places.sheetId;
      link.rel = "stylesheet";
      link.setAttribute("blocking", "render");
      link.href = sheet;
      document.head.append(link);
    }
  } catch (error) {
    // Blocked site data or a kept text that is not JSON: the page shows the device's look until the app runs.
    if (!(error instanceof DOMException || error instanceof SyntaxError)) {
      throw error;
    }
  }
}
