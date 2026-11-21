import { useEffect, useRef } from "react";

import { Button } from "./Button.tsx";
import { partOf } from "./parts.ts";
import { part } from "./tokens.ts";

export interface ShowMoreProps {
  /**
   * The words of the button: *Show more* and the plural of what the list holds (*Show more people*, *Show more
   * roles*), never *rows* and never the two words alone (UX §6; the catalog test of `@bugaboo/messages` holds every
   * text to it). A screen with two of them keeps them apart.
   */
  readonly label: string;
  /** The next page is on its way. */
  readonly pending: boolean;
  readonly onPress: () => void;
}

/**
 * How many frames *Show more* keeps itself in view after the focus came to it or the next page arrived: a list that
 * draws only the rows in the window measures them in the next frames and may move what is under it once more.
 */
const SETTLING_FRAMES = 10;

/** The keys that scroll the page or the box the button stands in: a person who presses one scrolls by hand. */
const SCROLL_KEYS: ReadonlySet<string> = new Set([
  "ArrowUp",
  "ArrowDown",
  "PageUp",
  "PageDown",
  "Home",
  "End",
]);

/** What a person does to scroll by hand, besides a key of `SCROLL_KEYS`: from then on the place of the page is theirs. */
const HAND_SCROLLS = ["wheel", "touchstart"] as const;

/** How a listener of the whole page listens: before the parts of the page, and without holding the scroll back. */
const LISTEN = { capture: true, passive: true } as const;

/** Words of the CSS standard: the focus of the keyboard, not of a pointer. */
const KEYBOARD_FOCUS = ":focus-visible";

/**
 * Keeps *Show more* in view for the keyboard while the page settles (WCAG 2.4.11). It follows only while the button
 * shows the focus of the keyboard, only after the focus came to it or it was pressed (until the next page is there, and
 * a few frames more), and it stops at once when the person scrolls by hand: a mouse that pressed the button, or a
 * person who scrolls away, is never pulled back (nothing moves under the cursor).
 */
class KeepInView {
  /** The frames left to follow; none while nothing is to be followed. */
  private frames = 0;
  /** A press waits for its page: the button is followed until the page is there. */
  private waiting = false;
  private frame = 0;
  private readonly row: HTMLElement;
  private readonly follow = new ResizeObserver(() => {
    this.bring();
  });

  constructor(row: HTMLElement) {
    this.row = row;
    const list = row.previousElementSibling;
    if (list instanceof Element) {
      this.follow.observe(list);
    }
    for (const event of HAND_SCROLLS) {
      document.addEventListener(event, this.byHand, {
        capture: true,
        passive: true,
      });
    }
  }

  /** The focus came to the button, or its page arrived: follow it for the frames of settling. */
  settle(): void {
    this.frames = SETTLING_FRAMES;
    this.waiting = false;
    this.step();
  }

  /** The button was pressed: follow it until its page is there (`settle`). */
  pressed(): void {
    this.waiting = true;
  }

  stop(): void {
    this.frames = 0;
    this.waiting = false;
    cancelAnimationFrame(this.frame);
  }

  dispose(): void {
    this.stop();
    this.follow.disconnect();
    for (const event of HAND_SCROLLS) {
      document.removeEventListener(event, this.byHand, LISTEN);
    }
    document.removeEventListener("keydown", this.byKey, LISTEN);
  }

  private readonly byHand = (): void => {
    this.stop();
  };

  private readonly byKey = (event: KeyboardEvent): void => {
    if (SCROLL_KEYS.has(event.key)) {
      this.stop();
    }
  };

  private step(): void {
    cancelAnimationFrame(this.frame);
    if (this.frames === 0) {
      return;
    }
    this.frames -= 1;
    this.bring();
    this.frame = requestAnimationFrame(() => {
      this.step();
    });
  }

  /** Brings the row into view, while it is followed and its button shows the focus of the keyboard. */
  private bring(): void {
    const focused = this.row.querySelector(KEYBOARD_FOCUS);
    if ((this.frames > 0 || this.waiting) && focused !== null) {
      this.row.scrollIntoView({ block: "nearest" });
    }
  }
}

/**
 * *Show more* under a list that is read a page at a time: one button, on a row of its own, on every screen. The list
 * above grows when a page arrives, or moves when it measures rows it had only guessed the height of; for a short while
 * after the keyboard brought the focus to the button or pressed it, the button is brought back into view, so the focus
 * of the keyboard is never out of sight (WCAG 2.4.11), and a person who scrolls is left where they scrolled to.
 */
export function ShowMore({ label, pending, onPress }: ShowMoreProps) {
  const row = useRef<HTMLDivElement>(null);
  const keep = useRef<KeepInView | null>(null);
  useEffect(() => {
    const kept = row.current === null ? null : new KeepInView(row.current);
    keep.current = kept;
    return () => {
      kept?.dispose();
    };
  }, []);
  // The page of a press is there: the list has grown, and measures its new rows in the next frames.
  const waited = useRef(pending);
  useEffect(() => {
    if (waited.current && !pending) {
      keep.current?.settle();
    }
    waited.current = pending;
  }, [pending]);
  return (
    <div
      ref={row}
      {...partOf(part.row)}
      onFocus={() => {
        keep.current?.settle();
      }}
      onBlur={() => {
        keep.current?.stop();
      }}
    >
      <Button
        isPending={pending}
        onPress={() => {
          keep.current?.pressed();
          onPress();
        }}
      >
        {label}
      </Button>
    </div>
  );
}

/** What of a paged query *Show more* needs: the shape of a TanStack infinite query. */
export interface PagedList {
  readonly hasNextPage: boolean;
  readonly isFetchingNextPage: boolean;
  readonly fetchNextPage: () => unknown;
}

/** *Show more* for a paged query: drawn only while there is a next page, and it reads the next page. */
export function ShowMoreOf({ label, list }: { readonly label: string; readonly list: PagedList }) {
  return list.hasNextPage ? (
    <ShowMore
      label={label}
      pending={list.isFetchingNextPage}
      onPress={() => {
        void list.fetchNextPage();
      }}
    />
  ) : null;
}
