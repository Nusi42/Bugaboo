// The book of a run: what each person kept, was refused and read. The driver writes it as JSON,
// and `bb-load-check` (crates/bb-load-check) judges it: the rules are there and nowhere else. The
// shape is the one of `bb_load_check::Book`; `tools/dev/load-book.example.json` is read by a test
// of each side, so the two cannot drift apart unseen.

/** An issue the people work on. */
export interface Work {
  readonly id: string;
  readonly key: string;
}

/** A change the server kept: the version the issue had after it, and the text of a save of the description. */
export interface Kept {
  readonly key: string;
  readonly version: number;
  readonly text?: string;
}

/** A version of an issue that a person read. */
export interface Seen {
  readonly person: number;
  readonly key: string;
  readonly version: number;
}

/** What became of the text of a draft: saved, refused (it stays as the draft), or the editor vanished. */
export type DraftEnd = "saved" | "refused" | "vanished";

/** A text sent as a draft with a heartbeat, and what became of it. */
export interface Drafted {
  readonly holder: string;
  readonly key: string;
  readonly text: string;
  readonly ending: DraftEnd;
  /** For a refused save: whether the person found the text as their draft at once. */
  readonly found_as_draft?: boolean;
}

/** What goes into the file for `bb-load-check`. */
export interface BookFile {
  readonly issues: readonly Work[];
  readonly kept: readonly Kept[];
  readonly comments: readonly string[];
  readonly seen: readonly Seen[];
  readonly drafted: readonly Drafted[];
  readonly broken: readonly string[];
}

/** The book, written by everybody at once, and two counts for the summary of the run. */
export interface Book {
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
  readonly file: (issues: readonly Work[]) => BookFile;
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
export function newBook(): Book {
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
