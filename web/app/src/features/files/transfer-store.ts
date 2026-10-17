/** Where files go: an issue, and a comment under it, or the issue itself. */
export interface TransferTarget {
  readonly issue: string;
  readonly comment: string | null;
}

/** What a file on its way is doing. */
export type TransferStatus = "waiting" | "sending" | "refused";

/** A file that is being sent, or that could not be sent and says why. */
export interface Transfer {
  readonly id: number;
  readonly target: TransferTarget;
  readonly name: string;
  readonly size: number;
  /** How many bytes the server has. */
  readonly sent: number;
  readonly status: TransferStatus;
  /** Why it was not sent, in words. */
  readonly message: string | null;
  /** Whether sending it again can help (the connection was lost). */
  readonly canRetry: boolean;
  /** The limit of the project, in bytes, for the sentence that refuses a file that is too big. */
  readonly limit: number;
}

/** The list of transfers and the files behind them, with the screens that watch the list. */
export interface TransferStore {
  readonly subscribe: (listener: () => void) => () => void;
  readonly snapshot: () => readonly Transfer[];
  /** The file a transfer sends. */
  readonly fileOf: (id: number) => File | undefined;
  /** The next number for a transfer. */
  readonly nextId: () => number;
  /** Adds transfers, with the file behind each one that is to be sent. */
  readonly append: (
    added: readonly Transfer[],
    chosen: readonly (readonly [number, File])[],
  ) => void;
  readonly replace: (id: number, changes: Partial<Transfer>) => void;
  readonly remove: (id: number) => void;
}

/** Makes the list of transfers: an immutable list that is replaced, so a screen can tell it changed. */
export function createTransferStore(): TransferStore {
  let transfers: readonly Transfer[] = [];
  const files = new Map<number, File>();
  const listeners = new Set<() => void>();
  let last = 0;
  const change = (update: (all: readonly Transfer[]) => readonly Transfer[]): void => {
    transfers = update(transfers);
    for (const listener of listeners) {
      listener();
    }
  };
  return {
    subscribe: (listener) => {
      listeners.add(listener);
      return () => {
        listeners.delete(listener);
      };
    },
    snapshot: () => transfers,
    fileOf: (id) => files.get(id),
    nextId: () => ++last,
    append: (added, chosen) => {
      for (const [id, file] of chosen) {
        files.set(id, file);
      }
      change((all) => [...all, ...added]);
    },
    replace: (id, changes) => {
      change((all) => all.map((each) => (each.id === id ? { ...each, ...changes } : each)));
    },
    remove: (id) => {
      files.delete(id);
      change((all) => all.filter((each) => each.id !== id));
    },
  };
}
