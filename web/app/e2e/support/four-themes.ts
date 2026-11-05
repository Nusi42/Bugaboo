import { type Page } from "@playwright/test";

import { chooseTheme, themes, type Theme } from "./checks.ts";
import { layoutOf } from "./layout.ts";
import { shootWindowAs, shotWholeAs } from "./screenshots.ts";
import { inOrder } from "./sequence.ts";
import { showTheme } from "./showcase.ts";

/** Where the pictures go, below the folder the run was given, unless a test names its own. */
const MILESTONE = "M4/workflows";

/**
 * Takes the open screen in the four themes, named for the screen and for the window it is shown in,
 * into the folder `milestone` below the one the run was given.
 */
export async function inFourThemes(page: Page, name: string, milestone = MILESTONE): Promise<void> {
  const layout = layoutOf(page);
  await inOrder(
    themes.map((theme) => async () => {
      await chooseTheme(page, theme);
      await shotWholeAs(page, `${name}-${layout}`, milestone, theme.id);
    }),
  );
}

/** Which screen is taken, where it goes, and the address of *My settings › Appearance*. */
interface ChosenShot {
  /** The picture is the window as it is, not grown to the height of the page. */
  readonly asWindow?: boolean;
  readonly name: string;
  readonly milestone: string;
  readonly appearance: string;
}

/** Takes the screen in each of `chosen`, picked in *My settings › Appearance* before `open` opens it. */
async function inChosen(
  page: Page,
  shot: ChosenShot,
  chosen: readonly Theme[],
  open: () => Promise<void>,
): Promise<void> {
  const layout = layoutOf(page);
  await inOrder(
    chosen.map((theme) => async () => {
      await showTheme(page, theme, shot.appearance);
      await open();
      await (shot.asWindow === true ? shootWindowAs : shotWholeAs)(
        page,
        `${shot.name}-${layout}`,
        shot.milestone,
        theme.id,
      );
    }),
  );
}

/**
 * The same for a screen that reads the person's appearance again when it opens (the issue page):
 * each theme is chosen in *My settings › Appearance* as a person does, then `open` opens the screen.
 */
export async function inFourChosenThemes(
  page: Page,
  name: string,
  milestone: string,
  appearance: string,
  open: () => Promise<void>,
): Promise<void> {
  await inChosen(page, { name, milestone, appearance }, themes, open);
}

/** The first light and the last dark theme, the list's two ends: enough to see a screen in both kinds. */
const lightAndDark = themes.filter((_, at) => at === 0 || at === themes.length - 1);

/** As `inFourChosenThemes`, in one light and one dark theme. */
export async function inLightAndDarkChosen(
  page: Page,
  name: string,
  milestone: string,
  appearance: string,
  open: () => Promise<void>,
): Promise<void> {
  await inChosen(page, { name, milestone, appearance }, lightAndDark, open);
}

/** As `inFourChosenThemes`, taking the window as it is: for a screen that depends on the height of the window. */
export async function inFourChosenThemesAsWindow(
  page: Page,
  name: string,
  milestone: string,
  appearance: string,
  open: () => Promise<void>,
): Promise<void> {
  await inChosen(page, { name, milestone, appearance, asWindow: true }, themes, open);
}
