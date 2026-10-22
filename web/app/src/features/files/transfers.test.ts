import { ApiError } from "@bugaboo/api-client";
import { describe, expect, it, vi } from "vitest";

import { fakeUploads, lostConnection, problem } from "./fake-uploads.ts";
import { waitsBetweenTries } from "./transfer.ts";
import { createTransfers, type Transfer, type Transfers } from "./transfers.ts";

const target = { issue: "WEB-1", comment: null } as const;
const LIMIT = 25 * 1024 * 1024;

/** Transfers on a fake server that never waits; `refreshed` counts the reads of the list after a file. */
function setUp(failures: readonly (Readonly<ApiError> | undefined)[] = []) {
  const server = fakeUploads(failures);
  const refreshed = vi.fn<() => void>();
  const transfers = createTransfers({
    transport: server.transport,
    wait: () => Promise.resolve(),
    refresh: refreshed,
  });
  return { server, transfers, refreshed };
}

const file = (): File => new File(["0123456789"], "a.txt");

/** The one transfer there is. */
function theTransfer(transfers: Transfers): Transfer {
  const [first] = transfers.snapshot();
  if (first === undefined) {
    throw new Error("there is no transfer");
  }
  return first;
}

/** A server that loses the connection for good after the first part. */
function lostAfterTheFirstPart(): readonly (Readonly<ApiError> | undefined)[] {
  const lost = lostConnection();
  return [undefined, ...waitsBetweenTries.map(() => lost), lost];
}

const isRefused = (transfers: Transfers) => () => {
  expect(theTransfer(transfers).status).toBe("refused");
};

describe("att_01 files on their way", () => {
  it("sends a file and takes it off the list, and the list of files is read again", async () => {
    const { transfers, refreshed, server } = setUp();
    transfers.add(target, [file()], LIMIT);
    await vi.waitFor(() => {
      expect(refreshed).toHaveBeenCalledTimes(1);
    });
    expect(transfers.snapshot()).toEqual([]);
    expect(server.sent).toEqual([0, 4, 8]);
  });

  it("refuses an empty file in words, with no upload to throw away", () => {
    const { transfers, server } = setUp();
    transfers.add(target, [new File([], "empty.txt")], LIMIT);
    expect(theTransfer(transfers).status).toBe("refused");
    transfers.dismiss(theTransfer(transfers).id);
    expect(server.cancelled).toEqual([]);
    expect(server.started()).toBe(0);
  });
});

describe("att_01 a transfer that was lost", () => {
  it("is kept with its upload, and goes on with that upload when tried again", async () => {
    const { transfers, server } = setUp(lostAfterTheFirstPart());
    transfers.add(target, [file()], LIMIT);
    await vi.waitFor(isRefused(transfers));
    const lost = theTransfer(transfers);
    expect(lost.canRetry).toBe(true);
    expect(lost.upload).toBe("u1");
    expect(server.sent).toEqual([0]);

    transfers.retry(lost.id);
    await vi.waitFor(() => {
      expect(transfers.snapshot()).toEqual([]);
    });
    // The rest went to the same upload: nothing was sent twice and no second upload was started.
    expect(server.started()).toBe(1);
    expect(server.sent).toEqual([0, 4, 8]);
    expect(server.cancelled).toEqual([]);
  });

  it("throws its upload away when it is closed", async () => {
    const { transfers, server } = setUp(lostAfterTheFirstPart());
    transfers.add(target, [file()], LIMIT);
    await vi.waitFor(isRefused(transfers));
    transfers.dismiss(theTransfer(transfers).id);
    expect(transfers.snapshot()).toEqual([]);
    await vi.waitFor(() => {
      expect(server.cancelled).toEqual(["u1"]);
    });
  });
});

describe("att_01 a transfer that was refused", () => {
  it("throws its upload away at once when the refusal leaves it open", async () => {
    const full = new ApiError(507, problem("file.no_room", 507));
    const { transfers, server } = setUp([full]);
    transfers.add(target, [file()], LIMIT);
    await vi.waitFor(isRefused(transfers));
    expect(theTransfer(transfers).canRetry).toBe(false);
    await vi.waitFor(() => {
      expect(server.cancelled).toEqual(["u1"]);
    });
    // The upload is gone from the transfer too: closing it asks the server for nothing more.
    expect(theTransfer(transfers).upload).toBeNull();
    transfers.dismiss(theTransfer(transfers).id);
    expect(server.cancelled).toEqual(["u1"]);
  });
});
