import { maskOf, settle } from "./showcase-masks.ts";
import { type Browser, type Page } from "@playwright/test";

import { chooseTheme, type Theme } from "./checks.ts";
import { signIn } from "./flows.ts";
import { type Instance } from "./server.ts";
import { inOrder } from "./sequence.ts";
import { personOf, showcaseIds, type ShowcaseIds, type Who } from "./showcase-sample.ts";
import { screens } from "./showcase-screens.ts";
import {
  type Device,
  onlyScreens,
  openDevice,
  type Screen,
  showTheme,
  takePicture,
} from "./showcase.ts";
import { paths } from "../../src/shared/paths.ts";

/** The people of the tour in the order it shows them, those who are not signed in first. */
const tourOrder: readonly Who[] = ["nobody", "administrator", "teamMember", "contributor", "guest"];

/** A screen with its place in the tour, which is its number in the file names. */
interface Stop {
  readonly screen: Screen;
  readonly order: number;
}

/** The screens one person sees in one window. */
function stopsOf(who: Who, device: Device): readonly Stop[] {
  return screens
    .map((screen, index): Stop => ({ screen, order: index + 1 }))
    .filter((stop) => stop.screen.who === who && (stop.screen.only ?? device) === device)
    .filter((stop) => onlyScreens?.includes(stop.screen.slug) ?? true);
}

/** What it takes for a person to see a screen in a theme: how they get in, and how the theme shows. */
interface Way {
  /** Gets the person to the point where screens can be opened. */
  readonly start: () => Promise<void>;
  /** Makes the open screen show the theme. */
  readonly show: () => Promise<void>;
}

/**
 * Nobody is signed in and picks the theme with the chooser at the bottom of each page; a person
 * signs in and keeps the theme as their choice in *My settings*.
 */
function wayOf(who: Who, page: Page, theme: Theme): Way {
  if (who === "nobody") {
    return { start: () => Promise.resolve(), show: () => chooseTheme(page, theme) };
  }
  return {
    start: async () => {
      await signIn(page, personOf[who]());
      await showTheme(page, theme, paths.appearance);
    },
    show: () => Promise.resolve(),
  };
}

/** Shows the screens of one person, in a browser of its own that is the window of the device. */
async function tourOf(
  browser: Browser,
  server: Instance,
  ids: ShowcaseIds,
  theme: Theme,
  device: Device,
  who: Who,
): Promise<void> {
  const context = await openDevice(browser, server, device);
  const page = await context.newPage();
  const way = wayOf(who, page, theme);
  await way.start();
  await inOrder(
    stopsOf(who, device).map(({ screen, order }) => async () => {
      await screen.open({ page, ids });
      await way.show();
      await settle(screen.slug, page);
      await takePicture(page, {
        order,
        screen: screen.slug,
        title: screen.title,
        theme,
        device,
        mask: maskOf(screen.slug, page),
      });
    }),
  );
  await context.close();
}

/** Takes every screen of the tour in one theme and one window, as every person sees it. */
export async function takeTour(
  browser: Browser,
  server: Instance,
  theme: Theme,
  device: Device,
): Promise<void> {
  const ids = await showcaseIds(server);
  await inOrder(tourOrder.map((who) => () => tourOf(browser, server, ids, theme, device, who)));
}
