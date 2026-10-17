import { ApiError } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { useMemo, useSyncExternalStore } from "react";

import { messageOf } from "../../shared/errors.ts";
import { connection } from "./connection.ts";
import { refreshFiles } from "./files.ts";
import { formatSize } from "./size.ts";
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
  readonly refresh: () => Promise<void>;
}

/** The files on their way: one store for the whole app, so a transfer outlives its screen. */
export interface Transfers {
  /** Takes files to send to a target, one after the other. `limit` is the limit of the project in bytes. */
  readonly add: (target: TransferTarget, files: readonly File[], limit: number) => void;
  /** Forgets a transfer that was refused or lost. */
  readonly dismiss: (id: number) => void;
  /** Sends a transfer that was lost again. */
  readonly retry: (id: number) => void;
  readonly subscribe: TransferStore["subscribe"];
  readonly snapshot: TransferStore["snapshot"];
}

/** Why a transfer ended in words: the limit of the project, a lost connection, or the server's reason. */
function messageFor(error: unknown, transfer: Transfer): string {
  if (error instanceof ApiError && error.problem?.code === "file.too_big") {
    return t("files.too-big", { limit: formatSize(transfer.limit) });
  }
  if (isTemporary(error)) {
    return t("files.lost", { name: transfer.name });
  }
  return messageOf(error);
}

/** A transfer for a file that was chosen; an empty file is refused at once, in words. */
function transferOf(id: number, target: TransferTarget, file: File, limit: number): Transfer {
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
    limit,
  };
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
      },
    );
    store.remove(transfer.id);
    await deps.refresh();
  } catch (error) {
    store.replace(transfer.id, {
      status: "refused",
      message: messageFor(error, transfer),
      canRetry: isTemporary(error),
    });
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
    add: (target, chosen, limit) => {
      const added = chosen.map((file) => transferOf(store.nextId(), target, file, limit));
      const sendable = added.flatMap((each, at) => {
        const file = chosen[at];
        return file === undefined || each.status === "refused" ? [] : [[each.id, file] as const];
      });
      store.append(added, sendable);
      start();
    },
    dismiss: store.remove,
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
  refresh: refreshFiles,
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
