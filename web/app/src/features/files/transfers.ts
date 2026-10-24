import { ApiError } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { useCallback, useMemo, useSyncExternalStore } from "react";

import { messageOf } from "../../shared/errors.ts";
import { refreshIssueFiles } from "../../shared/query-keys.ts";
import { connection } from "./connection.ts";
import { isTemporary, sendFile, type Transport } from "./transfer.ts";
import {
  createTransferStore,
  type Transfer,
  type TransferStore,
  type TransferTarget,
} from "./transfer-store.ts";

export type { Transfer, TransferStatus, TransferTarget } from "./transfer-store.ts";

/** What the transfers need from outside: the server, a way to wait, and what to do afterwards. */
export interface Dependencies {
  readonly transport: Transport;
  readonly wait: (milliseconds: number) => Promise<void>;
  readonly refresh: () => void;
}

/** The files on their way: one store for the whole app, so a transfer outlives its screen. */
export interface Transfers {
  /** Takes files to send to a target, one after the other. */
  readonly add: (target: TransferTarget, files: readonly File[]) => void;
  /** Forgets a transfer that was refused or lost, and throws its upload away. */
  readonly dismiss: (id: number) => void;
  /** Sends a transfer that was lost again, from where its upload stands. */
  readonly retry: (id: number) => void;
  readonly subscribe: TransferStore["subscribe"];
  readonly snapshot: TransferStore["snapshot"];
}

/** Whether the server refused the file for its size. */
function isTooBig(error: unknown): boolean {
  return error instanceof ApiError && error.problem?.code === "file.too-big";
}

/** Why a transfer ended in words: a lost connection, or the server's reason (a file that is too big is worded where it is shown). */
function messageFor(error: unknown, transfer: Transfer): string | null {
  if (isTooBig(error)) {
    return null;
  }
  if (isTemporary(error)) {
    return t("files.lost", { name: transfer.name });
  }
  return messageOf(error);
}

/** A transfer for a file that was chosen; an empty file is refused at once, in words. */
function transferOf(id: number, target: TransferTarget, file: File): Transfer {
  const empty = file.size === 0;
  return {
    id,
    target,
    name: file.name,
    size: file.size,
    sent: 0,
    status: empty ? "refused" : "waiting",
    message: empty ? t("files.empty", { name: file.name }) : null,
    canRetry: false,
    tooBig: false,
    upload: null,
  };
}

/** What a call that does not matter to anybody does when it fails: nothing. */
function ignored(): void {
  // The server does not mind an upload that is gone, and neither does the person.
}

/** Throws an upload away, if there is one. */
function throwAway(deps: Dependencies, upload: string | null): void {
  if (upload !== null) {
    deps.transport.cancel(upload).catch(ignored);
  }
}

/** Says in the record of a transfer why it ended, and keeps or throws away what the server has. */
function refuse(
  deps: Dependencies,
  store: TransferStore,
  transfer: Transfer,
  error: unknown,
): void {
  const again = isTemporary(error);
  // Where trying again can help, the upload stays for it to go on with. Any other refusal ends
  // it: what the server has of the file is of no use to anybody.
  const open = store.snapshot().find((each) => each.id === transfer.id)?.upload ?? null;
  if (!again) {
    throwAway(deps, open);
  }
  store.replace(transfer.id, {
    status: "refused",
    message: messageFor(error, transfer),
    tooBig: isTooBig(error),
    canRetry: again,
    upload: again ? open : null,
  });
}

/** Sends one transfer, and says in its record how it went. */
async function sendOne(
  deps: Dependencies,
  store: TransferStore,
  transfer: Transfer,
  file: File,
): Promise<void> {
  store.replace(transfer.id, {
    status: "sending",
    sent: 0,
    message: null,
    canRetry: false,
  });
  try {
    await sendFile(
      deps.transport,
      {
        issue: transfer.target.issue,
        comment: transfer.target.comment,
        name: transfer.name,
        size: transfer.size,
      },
      file,
      {
        wait: deps.wait,
        progress: (sent) => {
          store.replace(transfer.id, { sent });
        },
        started: (upload) => {
          store.replace(transfer.id, { upload });
        },
      },
      transfer.upload ?? undefined,
    );
    store.remove(transfer.id);
    deps.refresh();
  } catch (error) {
    refuse(deps, store, transfer, error);
  }
}

/** Makes the transfers of the app around a store, a server and a clock. */
export function createTransfers(deps: Dependencies): Transfers {
  const store = createTransferStore();
  let running = false;

  /** Sends the transfers that wait, one after the other, and stops when none waits. */
  const run = async (): Promise<void> => {
    const next = store.snapshot().find((each) => each.status === "waiting");
    const file = next === undefined ? undefined : store.fileOf(next.id);
    if (next === undefined || file === undefined) {
      running = false;
      return;
    }
    await sendOne(deps, store, next, file);
    return run();
  };
  const start = (): void => {
    if (!running) {
      running = true;
      void run();
    }
  };

  return {
    add: (target, chosen) => {
      const added = chosen.map((file) => transferOf(store.nextId(), target, file));
      const sendable = added.flatMap((each, at) => {
        const file = chosen[at];
        return file === undefined || each.status === "refused" ? [] : [[each.id, file] as const];
      });
      store.append(added, sendable);
      start();
    },
    dismiss: (id) => {
      throwAway(deps, store.snapshot().find((each) => each.id === id)?.upload ?? null);
      store.remove(id);
    },
    retry: (id) => {
      store.replace(id, { status: "waiting", message: null, canRetry: false });
      start();
    },
    subscribe: store.subscribe,
    snapshot: store.snapshot,
  };
}

/** The transfers of the app, through the one connection. */
export const transfers: Transfers = createTransfers({
  transport: connection,
  wait: (milliseconds) =>
    new Promise((done) => {
      setTimeout(done, milliseconds);
    }),
  refresh: refreshIssueFiles,
});

/** The files on their way to one target, as the screen of that target shows them. */
export function useTransfers(target: TransferTarget): readonly Transfer[] {
  const all = useSyncExternalStore(transfers.subscribe, transfers.snapshot);
  const { issue, comment } = target;
  return useMemo(
    () => all.filter((each) => each.target.issue === issue && each.target.comment === comment),
    [all, issue, comment],
  );
}

/**
 * Where files go when a person adds them to an issue (`comment` is `null`) or to a comment: the one
 * way an upload starts, for *Upload*, a drop, a paste and *Add files* under a comment.
 */
export function useAddFiles(
  issue: string,
  comment: string | null,
): (files: readonly File[]) => void {
  return useCallback(
    (files) => {
      transfers.add({ issue, comment }, files);
    },
    [issue, comment],
  );
}
