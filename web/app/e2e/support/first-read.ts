import { expect, type Page } from "@playwright/test";

import { type FirstMove, movedOnFirstRead, type Track, type View } from "./first-read-judge.ts";
import { readsOnTheirWay, type Box } from "./movement.ts";

export type { FirstMove } from "./first-read-judge.ts";

// The measure of the first read (UX §6.4, M8 WP-13): every part of a screen from the first frame of its page, until
// every read has answered and been drawn. The rule it is held to is `first-read-judge.ts`.

/** The key in the storage of the session that tells the next page of the tab to record its first read. */
const RECORD_KEY = "bugaboo-first-read";

/** The mark that ends the recording. */
const DONE_MARK = "data-first-read-done";

/** Where the page leaves what it recorded. */
const RECORDED = "bugabooFirstRead";

/** How many letters of its text name a part in a finding. */
const NAME_LETTERS = 32;

/** The pages that record their first reads already: the script is added once to each. */
const recording = new WeakSet<Page>();

/** What the page recorded: each part from the first frame, and the window. */
interface Recorded {
  readonly tracks: readonly Track[];
  readonly view: View;
}

/** What the page notes of one part: its name in a finding, and each change of its box. */
interface Noted {
  readonly name: string;
  readonly boxes: { frame: number; box: Readonly<Box> | null }[];
}

/**
 * What the page notes of its parts, frame by frame: where each part (`data-part`) stands, a change of its box, or that
 * it went or is hidden (laid out but hidden is not seen: it is drawn when it shows). Handed to the page with the script
 * below, so it uses nothing from outside.
 */
class FrameNotes {
  private readonly tracks = new Map<Element, Noted>();
  /** Whether each part draws something itself, as far as it was looked at. */
  private readonly painting = new WeakMap<Element, boolean>();
  private frame = 0;
  private readonly letters: number;

  constructor(letters: number) {
    this.letters = letters;
  }

  /** Notes the frame the browser drew last. */
  note(): void {
    const now = new Set<Element>();
    for (const element of document.querySelectorAll<HTMLElement | SVGElement>("[data-part]")) {
      const { x, y, width, height } = element.getBoundingClientRect();
      const seen = element.checkVisibility({ visibilityProperty: true }) && this.paints(element);
      const box = seen && width > 0 && height > 0 ? { x, y, width, height } : null;
      const known = this.tracks.get(element);
      if (box !== null || known !== undefined) {
        const track = known ?? {
          name: `${element.dataset.part ?? ""} #${String(this.tracks.size)} ${(element.textContent ?? "").trim().slice(0, this.letters)}`,
          boxes: [],
        };
        this.tracks.set(element, track);
        now.add(element);
        if (JSON.stringify(track.boxes.at(-1)?.box) !== JSON.stringify(box)) {
          track.boxes.push({ frame: this.frame, box });
        }
      }
    }
    for (const [element, track] of this.tracks) {
      if (!now.has(element) && track.boxes.at(-1)?.box !== null) {
        track.boxes.push({ frame: this.frame, box: null });
      }
    }
    this.frame += 1;
  }

  /**
   * Whether a part draws something itself, which a person could see move: text of its own, a picture or a control, a
   * background, a border or a shadow. A box that only holds other parts is seen by them. Looked at once per part.
   */
  private paints(element: HTMLElement | SVGElement): boolean {
    const known = this.painting.get(element);
    if (known !== undefined) {
      return known;
    }
    const style = getComputedStyle(element);
    let ownText = false;
    for (const child of element.childNodes) {
      ownText ||= child.nodeType === Node.TEXT_NODE && (child.textContent ?? "").trim() !== "";
    }
    const borders = [
      style.borderTopWidth,
      style.borderRightWidth,
      style.borderBottomWidth,
      style.borderLeftWidth,
    ];
    const drawn =
      ownText ||
      element.matches("img, svg, input, textarea, select, button, canvas, video") ||
      !/^rgba\(\d+, \d+, \d+, 0\)$|^transparent$/u.test(style.backgroundColor) ||
      borders.some((width) => width !== "0px") ||
      style.boxShadow !== "none";
    this.painting.set(element, drawn);
    return drawn;
  }

  /** Every part noted, with its changes. */
  noted(): readonly Noted[] {
    return [...this.tracks.values()];
  }
}

/**
 * The script that runs at the start of every page of the tab, before the app: when the test asked for it, it notes after
 * every frame where each part stands, until the mark is set. Each frame is read once the browser has drawn it (a
 * message after the frame's rendering), not in the frame's callback: there the layout is not yet what the browser
 * paints (a part measured by an observer is set after it).
 */
function recordFromTheFirstFrame([key, mark, out, letters]: readonly [
  string,
  string,
  string,
  number,
]): void {
  if (sessionStorage.getItem(key) === null) {
    return;
  }
  sessionStorage.removeItem(key);
  const notes = new FrameNotes(letters);
  const channel = new MessageChannel();
  const afterPaint = (): void => {
    requestAnimationFrame(() => {
      channel.port2.postMessage(null);
    });
  };
  channel.port1.addEventListener("message", () => {
    notes.note();
    if (document.documentElement.hasAttribute(mark)) {
      Reflect.set(globalThis, out, {
        tracks: notes.noted(),
        view: { width: innerWidth, height: innerHeight },
      });
      return;
    }
    afterPaint();
  });
  channel.port1.start();
  afterPaint();
}

/** Whether a value is a box, as the page wrote it down. */
function isBox(value: unknown): value is Box {
  return (
    typeof value === "object" &&
    value !== null &&
    ["x", "y", "width", "height"].every((key) => typeof Reflect.get(value, key) === "number")
  );
}

/** A field of a value the page wrote down, if it is an object. */
function fieldOf(value: unknown, key: string): unknown {
  return typeof value === "object" && value !== null ? Reflect.get(value, key) : undefined;
}

/** A list the page wrote down, or none. */
function listOf(value: unknown): readonly unknown[] {
  return Array.isArray(value) ? (value as readonly unknown[]) : [];
}

/** What the page wrote down, read back with each value checked. */
function recordedOf(value: unknown): Recorded {
  const view = fieldOf(value, "view");
  return {
    tracks: listOf(fieldOf(value, "tracks")).map((track) => ({
      name: String(fieldOf(track, "name")),
      boxes: listOf(fieldOf(track, "boxes")).map((each) => {
        const box = fieldOf(each, "box");
        return { frame: Number(fieldOf(each, "frame")), box: isBox(box) ? box : null };
      }),
    })),
    view: { width: Number(fieldOf(view, "width")), height: Number(fieldOf(view, "height")) },
  };
}

/**
 * Opens the address in the tab and gives every part that moved after it was first drawn, until the page has drawn
 * what it reads: its title, and no read of the API on its way. The tab must show a page of the app already (the
 * storage of the session tells the next page to record).
 */
export async function firstReadOf(page: Page, address: string): Promise<readonly FirstMove[]> {
  if (!recording.has(page)) {
    recording.add(page);
    const settings = JSON.stringify([RECORD_KEY, DONE_MARK, RECORDED, NAME_LETTERS]);
    await page.addInitScript({
      content: `${String(FrameNotes)}
(${String(recordFromTheFirstFrame)})(${settings});`,
    });
  }
  await page.evaluate((key) => {
    sessionStorage.setItem(key, "yes");
  }, RECORD_KEY);
  // The reads of the new page only: those of the page before are cut by the navigation, and its answers no longer count.
  await page.goto(address, { waitUntil: "commit" });
  const reads = readsOnTheirWay(page);
  await expect(page.locator("h1, [data-part='top-bar.title']").first()).toBeVisible();
  await reads.settled();
  await page.evaluate((mark) => {
    document.documentElement.setAttribute(mark, "");
  }, DONE_MARK);
  await expect
    .poll(() => page.evaluate((out) => Reflect.get(globalThis, out) !== undefined, RECORDED))
    .toBe(true);
  const recorded = recordedOf(
    await page.evaluate((out) => Reflect.get(globalThis, out) as unknown, RECORDED),
  );
  return movedOnFirstRead(recorded.tracks, recorded.view);
}
