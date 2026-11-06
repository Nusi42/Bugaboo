// The book of a run: what each person kept, was refused and read. The driver writes it as JSON,
// and `bb-load-check` (crates/bb-load-check) judges it: the rules are there and nowhere else. The
// shape of the file is declared once, in Rust (`book.rs`); `book.generated.ts` is made from it by
// `cargo xtask gen load-book`. This file holds what is not a declaration of that shape.

import type { Book, Dayed, Drafted, Kept, Searched, Seen, Work } from "./book.generated.ts";

export type {
  Book,
  Dayed,
  DraftEnd,
  Drafted,
  Kept,
  Searched,
  Seen,
  Work,
} from "./book.generated.ts";

/** The book, written by everybody at once, and two counts for the summary of the run. */
export interface BookWriter {
  readonly kept: (note: Kept) => void;
  readonly comment: (key: string) => void;
  /** A file was added to the issue with this readable ID. */
  readonly fileAdded: (key: string) => void;
  /** The person with this ID is a Guest: what they are told is judged by what a Guest may see. */
  readonly guest: (id: string) => void;
  /** The key of an own field that the run hid from Guests. */
  readonly hiddenFromGuests: (field: string) => void;
  readonly seen: (note: Seen) => void;
  /** A search, and what the person was told. */
  readonly searched: (note: Searched) => void;
  readonly drafted: (note: Drafted) => void;
  /** A stop of a timer saved time on the issue with this readable ID. */
  readonly timed: (key: string) => void;
  /** The server kept a change of a work day. */
  readonly dayed: (note: Dayed) => void;
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
  readonly files: number;
  readonly conflicts: number;
  readonly refused: number;
  readonly tookOver: number;
  readonly vanished: number;
  readonly searches: number;
  readonly timed: number;
  readonly days: number;
}

/** What the people wrote, as lists. */
interface Notes {
  readonly kept: Kept[];
  readonly comments: string[];
  readonly files: string[];
  readonly guests: string[];
  readonly hidden: string[];
  readonly seen: Seen[];
  readonly searched: Searched[];
  readonly drafted: Drafted[];
  readonly timed: string[];
  readonly days: Dayed[];
  readonly broken: string[];
}

/** A writer that adds a note to a list. */
function adder<T>(list: { readonly push: (note: T) => number }): (note: T) => void {
  return (note) => {
    list.push(note);
  };
}

/** The writers of the book that add a note to a list. */
type Writers = Pick<
  BookWriter,
  | "kept"
  | "comment"
  | "fileAdded"
  | "guest"
  | "hiddenFromGuests"
  | "seen"
  | "searched"
  | "drafted"
  | "timed"
  | "dayed"
  | "broken"
>;

/** Empty lists of what the people wrote, and the writers that add to them. */
function emptyNotes(): { readonly notes: Notes; readonly writers: Writers } {
  const notes: Notes = {
    kept: [],
    comments: [],
    files: [],
    guests: [],
    hidden: [],
    seen: [],
    searched: [],
    drafted: [],
    timed: [],
    days: [],
    broken: [],
  };
  const writers: Writers = {
    kept: adder(notes.kept),
    comment: adder(notes.comments),
    fileAdded: adder(notes.files),
    guest: adder(notes.guests),
    hiddenFromGuests: adder(notes.hidden),
    seen: adder(notes.seen),
    searched: adder(notes.searched),
    drafted: adder(notes.drafted),
    timed: adder(notes.timed),
    dayed: adder(notes.days),
    broken: adder(notes.broken),
  };
  return { notes, writers };
}

/** An empty book. */
export function newBook(): BookWriter {
  const { notes, writers } = emptyNotes();
  const events = { conflicts: 0, tookOver: 0 };
  return {
    ...writers,
    conflict: () => {
      events.conflicts += 1;
    },
    tookOver: () => {
      events.tookOver += 1;
    },
    file: (issues) => ({
      issues,
      kept: notes.kept,
      comments: notes.comments,
      seen: notes.seen,
      drafted: notes.drafted,
      broken: notes.broken,
      files: notes.files,
      guests: notes.guests,
      hidden_from_guests: notes.hidden,
      searched: notes.searched,
      timed: notes.timed,
      days: notes.days,
    }),
    counts: () => ({
      kept: notes.kept.length,
      comments: notes.comments.length,
      files: notes.files.length,
      conflicts: events.conflicts,
      refused: notes.drafted.filter((note) => note.ending === "refused").length,
      tookOver: events.tookOver,
      searches: notes.searched.length,
      vanished: notes.drafted.filter((note) => note.ending === "vanished").length,
      timed: notes.timed.length,
      days: notes.days.length,
    }),
  };
}
