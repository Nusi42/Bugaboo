/** An issue the people work on. */
export interface Work {
  readonly id: string;
  readonly key: string;
}

/** Something that happened in a run and that the invariants are about. */
export type Note =
  /** The server kept a change: the version the issue had after it, and the text if it was the description. */
  | {
      readonly kind: "kept";
      readonly key: string;
      readonly version: number;
      readonly text?: string;
    }
  | { readonly kind: "comment"; readonly key: string }
  /** The version a person read. */
  | {
      readonly kind: "seen";
      readonly person: number;
      readonly key: string;
      readonly version: number;
    }
  /** A save was refused (a lost lock, a take-over, a changed text) and its text was found as the draft. */
  | { readonly kind: "refused"; readonly code: string }
  /** A change that came second and was sent again on the newer version (*Use mine*). */
  | { readonly kind: "conflict" }
  | { readonly kind: "took-over" }
  /** An editor who went away without letting go: where, who, and since when the lock was theirs. */
  | {
      readonly kind: "abandoned";
      readonly work: Work;
      readonly holder: string;
      readonly since: string;
      readonly at: number;
    }
  /** Anything that broke a rule. */
  | { readonly kind: "broken"; readonly message: string };

/** The notes of a run, written by everybody at once. */
export interface Book {
  readonly note: (note: Note) => void;
  readonly all: () => readonly Note[];
}

/** An empty book. */
export function newBook(): Book {
  const notes: Note[] = [];
  return {
    note: (note) => {
      notes.push(note);
    },
    all: () => notes,
  };
}

/** The notes of one kind. */
export function ofKind<Kind extends Note["kind"]>(
  book: Book,
  kind: Kind,
): readonly Extract<Note, { readonly kind: Kind }>[] {
  return book
    .all()
    .filter((note): note is Extract<Note, { readonly kind: Kind }> => note.kind === kind);
}
