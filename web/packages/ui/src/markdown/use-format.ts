import { useEffect, useRef, type RefObject } from "react";

import { applyFormat, type FormatAction, type FormatWords } from "./format.ts";

/** Where the selection goes once the text has changed. */
interface Wanted {
  readonly text: string;
  readonly start: number;
  readonly end: number;
}

export interface Formatting {
  /** Goes on the text box, which the buttons read the selection from. */
  readonly area: RefObject<HTMLTextAreaElement | null>;
  /** What a button of the toolbar does to the text and the selection. */
  readonly format: (action: FormatAction) => void;
}

/**
 * The toolbar of the Markdown editor: a button changes the text (`applyFormat`), and once the text
 * box shows the new text its selection is set, so the person goes on typing where it makes sense.
 */
export function useFormatting(
  value: string,
  onChange: (value: string) => void,
  words: FormatWords,
): Formatting {
  const area = useRef<HTMLTextAreaElement>(null);
  const wanted = useRef<Wanted | null>(null);
  useEffect(() => {
    const next = wanted.current;
    if (next !== null && area.current !== null && next.text === value) {
      wanted.current = null;
      area.current.focus();
      area.current.setSelectionRange(next.start, next.end);
    }
  }, [value]);
  const format = (action: FormatAction): void => {
    const start = area.current?.selectionStart ?? value.length;
    const end = area.current?.selectionEnd ?? value.length;
    const edit = applyFormat({ text: value, start, end }, action, words);
    wanted.current = edit;
    onChange(edit.text);
  };
  return { area, format };
}
