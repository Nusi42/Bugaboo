import { doNothing } from "@bugaboo/ui";
import { useEffect, useLayoutEffect, useRef } from "react";

/** The words of the keys the box of Advanced search reacts to. */
const keyNames = {
  down: "ArrowDown",
  up: "ArrowUp",
  enter: "Enter",
  tab: "Tab",
  escape: "Escape",
} as const;

export interface BoxEvents {
  /** The list of offers is open. */
  readonly showing: boolean;
  readonly count: number;
  /** The offer the keys are on. */
  readonly active: number | undefined;
  readonly onActive: (next: number | undefined) => void;
  /** Takes the offer the keys are on. */
  readonly onTake: (index: number) => void;
  readonly onClose: () => void;
  /** The text was typed in: the whole text, and the place of the cursor. */
  readonly onInput: (text: string, cursor: number) => void;
  /** The cursor moved without typing (a key, a press). */
  readonly onMoved: (cursor: number) => void;
  readonly onFocus: () => void;
  readonly onBlur: () => void;
}

/** Where the choice goes after an arrow key: it runs around the ends of the list. */
function stepped(active: number | undefined, step: 1 | -1, count: number): number {
  const from = active ?? (step === 1 ? -1 : 0);
  return (from + step + count) % count;
}

/** What a key does in the box. */
function keyPressed(event: KeyboardEvent, events: BoxEvents): void {
  const { showing, count, active, onActive, onTake, onClose } = events;
  const takes = event.key === keyNames.enter || event.key === keyNames.tab;
  if (showing && (event.key === keyNames.down || event.key === keyNames.up)) {
    event.preventDefault();
    onActive(stepped(active, event.key === keyNames.down ? 1 : -1, count));
  } else if (showing && takes && active !== undefined) {
    event.preventDefault();
    onTake(active);
  } else if (event.key === keyNames.enter) {
    event.preventDefault();
  } else if (showing && event.key === keyNames.escape) {
    event.stopPropagation();
    onClose();
  }
}

/** Listens to the events of the box; gives the way to stop. */
function listen(
  readBox: () => HTMLTextAreaElement | null,
  latest: { readonly current: BoxEvents },
): () => void {
  const element = readBox();
  if (element === null) {
    return doNothing;
  }
  const onKeyDown = (event: KeyboardEvent): void => {
    keyPressed(event, latest.current);
  };
  const onInput = (): void => {
    latest.current.onInput(element.value, element.selectionStart);
  };
  const onMoved = (): void => {
    latest.current.onMoved(element.selectionStart);
  };
  const onFocus = (): void => {
    latest.current.onFocus();
  };
  const onBlur = (): void => {
    latest.current.onBlur();
  };
  element.addEventListener("keydown", onKeyDown);
  element.addEventListener("input", onInput);
  element.addEventListener("keyup", onMoved);
  element.addEventListener("pointerup", onMoved);
  element.addEventListener("focus", onFocus);
  element.addEventListener("blur", onBlur);
  return () => {
    element.removeEventListener("keydown", onKeyDown);
    element.removeEventListener("input", onInput);
    element.removeEventListener("keyup", onMoved);
    element.removeEventListener("pointerup", onMoved);
    element.removeEventListener("focus", onFocus);
    element.removeEventListener("blur", onBlur);
  };
}

/**
 * The events of the box (UX design section 6.2): the arrows move the choice in the list of
 * offers, Enter or Tab take it, Esc closes the list; Enter never makes a new line, because the
 * search is one line of the language. Gives the reference for the box; the events are listened
 * to on the element itself, like the other boxes of the app.
 */
export function useBoxEvents(events: BoxEvents) {
  const box = useRef<HTMLTextAreaElement>(null);
  const latest = useRef(events);
  useLayoutEffect(() => {
    latest.current = events;
  });
  useEffect(() => {
    return listen(() => box.current, latest);
  }, []);
  return box;
}
