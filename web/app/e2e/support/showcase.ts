import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { t } from "@bugaboo/messages";
import {
  type Browser,
  type BrowserContext,
  type BrowserContextOptions,
  expect,
  type Locator,
  type Page,
} from "@playwright/test";

import { pickTheme, type Theme, expectLook } from "./checks.ts";
import { sampleStartMs } from "./sample.ts";
import { type Instance } from "./server.ts";
import { type ShowcaseIds, type Who } from "./showcase-sample.ts";
import { screenshotFolder, shootWindowAs, shotWholeAs } from "./screenshots.ts";
import { viewports } from "../../../tooling/viewports.ts";

/**
 * Only these screens are taken when `BB_SHOWCASE_ONLY` names them (slugs, separated by commas): a
 * change to a few screens is looked at without taking the whole tour again.
 */
export const onlyScreens: readonly string[] | undefined =
  process.env["BB_SHOWCASE_ONLY"]?.split(",");

/** The folder of the showcase pictures inside the screenshot folder. */
export const SHOWCASE = "showcase";

/**
 * The windows of the showcase, by the name in the file names: a small phone and a computer
 * (`web/tooling/viewports.ts` holds the sizes).
 */
export const devices = {
  phone: viewports.phone,
  computer: viewports.computer,
} as const;

/** A window of the showcase. */
export type Device = keyof typeof devices;

/** The windows of the showcase, in the order of the file names. */
export const deviceNames: readonly Device[] = ["phone", "computer"];

/** What the gallery needs to know about one picture. */
export interface Picture {
  /** The name of the file, without the folder. */
  readonly file: string;
  /** The short name of the screen, as in the file name (`people`). */
  readonly screen: string;
  /** One plain sentence that says what the picture shows. */
  readonly title: string;
  readonly theme: Theme["id"];
  readonly device: Device;
}

/** The list of the pictures, for the gallery. */
const INDEX = "index.json";

/** The pictures taken so far by this run. */
const taken = new Map<string, Picture>();

/** What a window of its own may have beside its size: a sharper screen, less motion. */
export type DeviceOptions = Readonly<
  Pick<BrowserContextOptions, "deviceScaleFactor" | "reducedMotion">
>;

/** Opens a browser of its own for a window of the showcase: a phone has a touch screen. */
export function deviceContext(
  browser: Browser,
  server: Instance,
  device: Device,
  options: DeviceOptions = {},
): Promise<BrowserContext> {
  return browser.newContext({
    baseURL: server.url,
    ignoreHTTPSErrors: true,
    viewport: devices[device],
    hasTouch: device === "phone",
    ...options,
  });
}

/** Opens a browser of its own for a window of the showcase, on the sample, with the clock of the sample. */
export async function openDevice(
  browser: Browser,
  server: Instance,
  device: Device,
): Promise<BrowserContext> {
  const context = await deviceContext(browser, server, device);
  // The clock of the browser stands at the moment the sample was made: "3 days ago" is then the
  // same on every run, and what the tour makes itself is "now" (M2C-F104).
  await context.clock.setFixedTime(sampleStartMs());
  return context;
}

/** The two digits in front of a file name, which keep the pictures in the order of the tour. */
function numbered(order: number): string {
  return String(order).padStart(2, "0");
}

/** What is needed to name and describe a picture of an open screen. */
export interface Shot {
  /** The place of the screen in the tour, from 1. */
  readonly order: number;
  readonly screen: string;
  readonly title: string;
  readonly theme: Theme;
  readonly device: Device;
  /** What the server makes new on every run (a moment, a secret): painted over in the picture. */
  readonly mask?: readonly Locator[];
}

/**
 * The tall screens whose whole page is too small to read in a contact sheet: a computer also gets the two
 * windows' worth from the top of the page, at the size they have (M5 exit, O11).
 */
const withCrops: ReadonlySet<string> = new Set([
  "project-roles",
  "issue",
  "issue-panel",
  "issue-team-member",
]);

/** The tall screens a phone also gets the two windows' worth for: the page of an issue and its details (M5 exit, round 8). */
const withPhoneCrops: ReadonlySet<string> = new Set(["issue", "issue-details"]);

/** Waits until the browser has drawn what was just scrolled to, so a picture is never of an empty page. */
function painted(page: Page): Promise<unknown> {
  return page.evaluate(
    () =>
      new Promise((done) => {
        requestAnimationFrame(() => {
          requestAnimationFrame(done);
        });
      }),
  );
}

/** Scrolls the page to this height: the window on a computer, the main area of the page on a phone, where that is the box that scrolls. */
async function scrollPageTo(page: Page, top: number): Promise<void> {
  await page.evaluate((to) => {
    window.scrollTo(0, to);
    document.querySelector("[data-part='app-layout.main']")?.scrollTo(0, to);
  }, top);
  await painted(page);
}

/** The pictures of the top and of the next part of a tall page, at the size of the window. */
async function takeCrops(page: Page, shot: Shot): Promise<void> {
  if (shot.screen === "project-move") {
    // The question with its body scrolled to the end, so the last words of what changes are seen whole.
    await page.locator("[data-part='confirm.body']").evaluate((node) => {
      node.scrollTop = node.scrollHeight;
    });
    await shootWindowAs(
      page,
      `${numbered(shot.order)}-${shot.screen}-${shot.device}-scrolled`,
      SHOWCASE,
      shot.theme.id,
    );
    return;
  }
  if (shot.screen === "new-issue-minimized" && shot.device === "computer") {
    // The bar of the minimized window, at the size it has, clear of the card behind it.
    await shootWindowAs(
      page,
      `${numbered(shot.order)}-${shot.screen}-${shot.device}-crop-bar`,
      SHOWCASE,
      shot.theme.id,
    );
    return;
  }
  const crops = shot.device === "computer" ? withCrops : withPhoneCrops;
  if (!crops.has(shot.screen)) {
    return;
  }
  const base = `${numbered(shot.order)}-${shot.screen}-${shot.device}`;
  await scrollPageTo(page, 0);
  await shootWindowAs(page, `${base}-crop-top`, SHOWCASE, shot.theme.id);
  await scrollPageTo(page, page.viewportSize()?.height ?? 0);
  await shootWindowAs(page, `${base}-crop-next`, SHOWCASE, shot.theme.id);
  await scrollPageTo(page, 0);
}

/** Takes the whole open screen in the theme it shows, and remembers it for the gallery. */
export async function takePicture(page: Page, shot: Shot): Promise<void> {
  const name = `${numbered(shot.order)}-${shot.screen}-${shot.device}`;
  await shotWholeAs(page, name, SHOWCASE, shot.theme.id, shot.mask);
  const file = `${name}-${shot.theme.id}.png`;
  taken.set(file, {
    file,
    screen: shot.screen,
    title: shot.title,
    theme: shot.theme.id,
    device: shot.device,
  });
  await takeCrops(page, shot);
}

/**
 * Chooses a theme in *My settings › Appearance*, as a person does, and waits until the choice is
 * kept: the chooser on the screen then matches the pictures taken after it.
 */
export async function showTheme(page: Page, theme: Theme, appearance: string): Promise<void> {
  await page.goto(appearance);
  await expect(page.getByRole("heading", { level: 1, name: t("appearance.title") })).toBeVisible();
  await pickTheme(page, theme);
  await expectLook(page, theme.id);
  await expect(page.getByRole("status").filter({ hasText: t("save.saved") })).toBeVisible();
}

/** Writes the list of the pictures taken so far for the gallery, in the order of the file names. */
export function writeIndex(): void {
  if (screenshotFolder === undefined) {
    return;
  }
  const folder = join(screenshotFolder, SHOWCASE);
  mkdirSync(folder, { recursive: true });
  const pictures = [...taken.values()].toSorted((a, b) => a.file.localeCompare(b.file));
  writeFileSync(join(folder, INDEX), `${JSON.stringify(pictures, null, 2)}\n`);
}

/** What a screen needs to open itself. */
export interface Needs {
  readonly page: Page;
  readonly ids: ShowcaseIds;
}

/** One screen of the tour. The tour shows them in the order of the table of screens. */
export interface Screen {
  /** The name in the file name. */
  readonly slug: string;
  /** One plain sentence for the gallery. */
  readonly title: string;
  readonly who: Who;
  /** Only in this window; the screen is in both windows when it is left out. */
  readonly only?: Device;
  /** Opens the screen, as a person gets there. */
  readonly open: (needs: Needs) => Promise<void>;
}
