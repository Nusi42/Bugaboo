import type { Answer, ReadableFields, Search, SearchLanguage } from "@bugaboo/wasm-search";
import { useEffect, useMemo, useRef, useState } from "react";

/** The two ways to write the same search. */
export const barMode = { simple: "simple", advanced: "advanced" } as const;
export type BarMode = (typeof barMode)[keyof typeof barMode];

export interface SearchBarInput {
  readonly language: SearchLanguage;
  readonly fields: ReadableFields;
  /** The canonical text the address holds. */
  readonly text: string;
  /** Writes the canonical text of the search into the address. */
  readonly onText: (canonical: string) => void;
}

export interface SearchBarState {
  readonly mode: BarMode;
  /** The text of the box in Advanced search: what was typed, or the text of the address. */
  readonly shown: string;
  /** What the checker finds in `shown`. */
  readonly answer: Answer;
  /** Whether the search can be shown as filter buttons: it has no mistake. */
  readonly canBeSimple: boolean;
  /** The tree of the search, for the filter buttons; `null` while the text has mistakes. */
  readonly search: Search | null;
  /** Advanced search types: a valid text goes to the address, a text with mistakes stays in the box. */
  readonly type: (text: string) => void;
  /** Simple search changed the tree: its canonical text goes to the address. */
  readonly change: (search: Search) => void;
  /** Switches; the search is the same on both sides. Returns whether it switched. */
  readonly switchTo: (mode: BarMode) => boolean;
}

/**
 * The text the bar works on: what the address holds, and what was typed in Advanced search until
 * it is valid. `write` sends a canonical text to the address once; when the address changes by
 * itself (back, a link) what was typed is dropped.
 */
function useTypedText(text: string, onText: (canonical: string) => void) {
  const [draft, setDraft] = useState<string | null>(null);
  const written = useRef(text);
  useEffect(() => {
    if (text !== written.current) {
      written.current = text;
      setDraft(null);
    }
  }, [text]);
  const write = (canonical: string): void => {
    if (canonical !== written.current) {
      written.current = canonical;
      onText(canonical);
    }
  };
  return { shown: draft ?? text, draft, setDraft, write };
}

/**
 * The state of one search bar. The address holds the canonical text; the box of Advanced search
 * holds what was typed until the text is valid, and then the address follows it. Simple and
 * Advanced search edit the same tree (ADR-0005), so switching only changes how it is shown.
 */
export function useSearchBar({ language, fields, text, onText }: SearchBarInput): SearchBarState {
  const [mode, setMode] = useState<BarMode>(barMode.simple);
  const { shown, draft, setDraft, write } = useTypedText(text, onText);
  const answer = useMemo(() => language.check(shown, fields), [language, fields, shown]);
  const clean = answer.mistakes.length === 0;
  // An address that holds a text in another spelling (a link, an old address) is written again
  // in the canonical one, so the address is the canonical text (ADR-0005, decision 4).
  const canonical = clean && draft === null ? answer.canonical : null;
  useEffect(() => {
    if (canonical !== null && canonical !== text) {
      write(canonical);
    }
  });
  return {
    // A text that cannot be shown as buttons is shown as text, whatever was chosen.
    mode: clean ? mode : barMode.advanced,
    shown,
    answer,
    canBeSimple: clean,
    search: clean ? answer.search : null,
    type: (next) => {
      setDraft(next);
      const checked = language.check(next, fields);
      if (checked.mistakes.length === 0 && checked.canonical !== null) {
        write(checked.canonical);
      }
    },
    change: (search) => {
      setDraft(null);
      write(language.print(search));
    },
    switchTo: (next) => {
      if (next === barMode.simple && !clean) {
        return false;
      }
      setMode(next);
      setDraft(null);
      return true;
    },
  };
}
