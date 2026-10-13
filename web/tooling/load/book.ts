// The book of a run: what each person kept, was refused and read. The driver writes it as JSON,
// and `bb-load-check` (crates/bb-load-check) judges it: the rules are there and nowhere else. The
// shape of the file is declared once, in Rust (`book.rs`); `book.generated.ts` is made from it by
// `cargo xtask gen load-book`. This file holds what is not a declaration of that shape.

import type { Book, Drafted, Kept, Seen, Work } from "./book.generated.ts";

export type { Book, DraftEnd, Drafted, Kept, Seen, Work } from "./book.generated.ts";

/** The book, written by everybody at once, and two counts for the summary of the run. */
export interface BookWriter {
  readonly kept: (note: Kept) => void;
  readonly comment: (key: string) => void;
  readonly seen: (note: Seen) => void;
  readonly drafted: (note: Drafted) => void;
  readonly broken: (message: string) => void;
  /** A change that came second and was sent again on the newer version (*Use mine*). */
  readonly conflict: () => void;
  /** A person took the editing over (*Edit anyway*). */
  readonly tookOver: () => void;
  /** The file for `bb-load-check`. */
  readonly file: (issues: readonly Work[]) => Book;
  /** How many of each there are, for the summary. */
  readonly counts: () => Counts;
}

/** The counts of the summary of a run. */
export interface Counts {
  readonly kept: number;
  readonly comments: number;
  readonly conflicts: number;
  readonly refused: number;
  readonly tookOver: number;
  readonly vanished: number;
}

/** An empty book. */
export function newBook(): BookWriter {
  const kept: Kept[] = [];
  const comments: string[] = [];
  const seen: Seen[] = [];
  const drafted: Drafted[] = [];
  const broken: string[] = [];
  const events = { conflicts: 0, tookOver: 0 };
  return {
    kept: (note) => {
      kept.push(note);
    },
    comment: (key) => {
      comments.push(key);
    },
    seen: (note) => {
      seen.push(note);
    },
    drafted: (note) => {
      drafted.push(note);
    },
    broken: (message) => {
      broken.push(message);
    },
    conflict: () => {
      events.conflicts += 1;
    },
    tookOver: () => {
      events.tookOver += 1;
    },
    file: (issues) => ({ issues, kept, comments, seen, drafted, broken }),
    counts: () => ({
      kept: kept.length,
      comments: comments.length,
      conflicts: events.conflicts,
      refused: drafted.filter((note) => note.ending === "refused").length,
      tookOver: events.tookOver,
      vanished: drafted.filter((note) => note.ending === "vanished").length,
    }),
  };
}
