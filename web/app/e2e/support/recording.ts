import { mkdirSync, writeFileSync } from "node:fs";
import { basename, join } from "node:path";

import { type Locator, type Page } from "@playwright/test";

import { type Frame, drawPointer, encodeGif } from "./gif.ts";
import { glide, type Point } from "./glide.ts";
import { readPng } from "./png.ts";
import { pointerAway } from "./screenshots.ts";
import { inOrder } from "./sequence.ts";

/** How long one picture of a movement shows. */
const STEP_MS = 40;

/** How long a picture shows when the screen is looked at for a moment. */
const LOOK_MS = 700;

/** Where the pointer rests before it first moves, as a share of the width and the height of the window. */
const REST = { x: 0.72, y: 0.9 } as const;

/** The pause between two typed letters, in pictures: a picture is taken every few letters. */
const LETTERS_PER_PICTURE = 2;

const CHANNELS = 4;

/** Where the screenshots of the recordings are also written one by one, when it is set. */
const framesFolder = process.env["BB_README_FRAMES"];

/** The pixels of a screenshot of the browser as red, green, blue and alpha bytes. */
export function rgbaOf(screenshot: Buffer): { width: number; height: number; rgba: Buffer } {
  const { width, channels, data } = readPng(screenshot);
  const height = data.length / channels / width;
  const rgba = Buffer.alloc(width * height * CHANNELS, 255);
  for (let pixel = 0; pixel < width * height; pixel += 1) {
    for (let channel = 0; channel < 3; channel += 1) {
      rgba[pixel * CHANNELS + channel] = data[pixel * channels + channel] ?? 0;
    }
  }
  return { width, height, rgba };
}

/**
 * A recording of a page: pictures of the window taken between the steps a script does, with a
 * small drawn arrow where the pointer is, saved as one GIF. The page does what the script says, for
 * real. While the arrow glides to what is pressed next, the real pointer waits off the page, so no
 * row it passes lights up; it comes onto the page at the end of the way, and only what it points
 * at shows its hover look. Every picture is taken with what moves on the page brought to its end
 * (`animations: "disabled"`), so no picture shows a window half faded in.
 */
export class Recording {
  readonly #page: Page;
  readonly #frames: Frame[] = [];
  /** The screenshots as they came, kept only to be looked at one by one (`BB_README_FRAMES`). */
  readonly #shots: Buffer[] = [];
  #at: Point;

  public constructor(page: Page) {
    this.#page = page;
    const size = page.viewportSize() ?? { width: 0, height: 0 };
    this.#at = { x: size.width * REST.x, y: size.height * REST.y };
  }

  /** Takes a picture of the window as it is now. */
  public async picture(delayMs = STEP_MS): Promise<void> {
    const shot = await this.#page.screenshot({ type: "png", animations: "disabled" });
    if (framesFolder !== undefined) {
      this.#shots.push(shot);
    }
    const { width, height, rgba } = rgbaOf(shot);
    const frame: Frame = { width, height, rgba, delayMs };
    const scale = width / (this.#page.viewportSize()?.width ?? width);
    drawPointer(frame, this.#at.x, this.#at.y, scale);
    this.#frames.push(frame);
  }

  /** Lets the screen be looked at for a moment. */
  public look(delayMs = LOOK_MS): Promise<void> {
    return this.picture(delayMs);
  }

  /** Glides the pointer to the middle of an element, with a picture of each step. */
  public async pointAt(target: Locator): Promise<void> {
    const box = await target.boundingBox();
    if (box === null) {
      throw new Error("The element to point at is not on the screen.");
    }
    const to = { x: box.x + box.width / 2, y: box.y + box.height / 2 };
    await pointerAway(this.#page);
    await inOrder(
      glide(this.#at, to).map((next) => async () => {
        this.#at = next;
        await this.picture();
      }),
    );
    await this.#page.mouse.move(to.x, to.y);
    await this.picture();
  }

  /**
   * Points at an element and presses it. No picture is taken right after: what the press opens is
   * still on its way, so the script waits for it and then looks.
   */
  public async click(target: Locator): Promise<void> {
    await this.pointAt(target);
    await target.click();
  }
  /** Types into an element, with a picture every few letters (each `settleMs` after its letter). */
  public async type(target: Locator, text: string, settleMs = 0): Promise<void> {
    await target.focus();
    await inOrder(
      Array.from(text).map((letter, number) => async () => {
        await this.#page.keyboard.type(letter);
        if ((number + 1) % LETTERS_PER_PICTURE === 0) {
          await this.#settledPicture(settleMs);
        }
      }),
    );
    await this.#settledPicture(settleMs);
  }

  /**
   * A picture once what a typed letter started has had `settleMs` to come (the results of a
   * search), so no picture shows the results of the letters before greyed out.
   */
  async #settledPicture(settleMs: number): Promise<void> {
    if (settleMs > 0) {
      await this.#page.waitForTimeout(settleMs);
    }
    await this.picture();
  }

  /**
   * Writes the recording as a GIF. With `BB_README_FRAMES` set, also every screenshot as it came,
   * as `<that folder>/<name of the GIF>/<number>.png`, to look at a recording one picture at a time.
   */
  public save(file: string): void {
    writeFileSync(file, encodeGif(this.#frames));
    if (framesFolder !== undefined) {
      const folder = join(framesFolder, basename(file, ".gif"));
      mkdirSync(folder, { recursive: true });
      this.#shots.forEach((shot, number) => {
        writeFileSync(join(folder, `${String(number).padStart(3, "0")}.png`), shot);
      });
    }
  }
}
