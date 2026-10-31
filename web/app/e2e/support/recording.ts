import { writeFileSync } from "node:fs";

import { type Locator, type Page } from "@playwright/test";

import { type Frame, drawPointer, encodeGif } from "./gif.ts";
import { readPng } from "./png.ts";
import { inOrder } from "./sequence.ts";

/** How long one picture of a movement shows. */
const STEP_MS = 40;

/** How long a picture shows when the screen is looked at for a moment. */
const LOOK_MS = 700;

/** How many pictures the pointer takes to go from one place to another. */
const MOVE_STEPS = 8;

/** The pause between two typed letters, in pictures: a picture is taken every few letters. */
const LETTERS_PER_PICTURE = 2;

const CHANNELS = 4;

/** The pixels of a screenshot of the browser as red, green, blue and alpha bytes. */
function rgbaOf(screenshot: Buffer): { width: number; height: number; rgba: Buffer } {
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
 * small drawn arrow where the pointer is, saved as one GIF. The page does what the script says,
 * for real: the pointer moves over it, so a control shows its hover look.
 */
export class Recording {
  readonly #page: Page;
  readonly #frames: Frame[] = [];
  #at: { readonly x: number; readonly y: number } | undefined;

  public constructor(page: Page) {
    this.#page = page;
  }

  /** Takes a picture of the window as it is now. */
  public async picture(delayMs = STEP_MS): Promise<void> {
    const { width, height, rgba } = rgbaOf(await this.#page.screenshot({ type: "png" }));
    const frame: Frame = { width, height, rgba, delayMs };
    if (this.#at !== undefined) {
      drawPointer(frame, this.#at.x, this.#at.y);
    }
    this.#frames.push(frame);
  }

  /** Lets the screen be looked at for a moment. */
  public look(delayMs = LOOK_MS): Promise<void> {
    return this.picture(delayMs);
  }

  /** Moves the pointer to the middle of an element, in a few steps, with a picture of each. */
  public async pointAt(target: Locator): Promise<void> {
    const box = await target.boundingBox();
    if (box === null) {
      throw new Error("The element to point at is not on the screen.");
    }
    const [x, y] = [box.x + box.width / 2, box.y + box.height / 2];
    const from = this.#at ?? { x: x + 120, y: y + 80 };
    await inOrder(
      Array.from({ length: MOVE_STEPS }, (_, number) => async () => {
        const share = (number + 1) / MOVE_STEPS;
        // Slow down at the end, as a hand does.
        const eased = 1 - (1 - share) ** 2;
        const next = { x: from.x + (x - from.x) * eased, y: from.y + (y - from.y) * eased };
        await this.#page.mouse.move(next.x, next.y);
        this.#at = next;
        await this.picture();
      }),
    );
  }

  /** Points at an element and presses it. */
  public async click(target: Locator): Promise<void> {
    await this.pointAt(target);
    await target.click();
    await this.picture();
  }

  /** Types into an element, with a picture every few letters. */
  public async type(target: Locator, text: string): Promise<void> {
    await target.focus();
    await inOrder(
      Array.from(text).map((letter, number) => async () => {
        await this.#page.keyboard.type(letter);
        if ((number + 1) % LETTERS_PER_PICTURE === 0) {
          await this.picture();
        }
      }),
    );
    await this.picture();
  }

  /** Writes the recording as a GIF. */
  public save(file: string): void {
    writeFileSync(file, encodeGif(this.#frames));
  }
}
