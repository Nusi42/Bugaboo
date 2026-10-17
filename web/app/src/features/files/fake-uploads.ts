import { ApiError, type ErrorCode, type Problem } from "@bugaboo/api-client";

import type { Attachment, Course, Delivery, Transport, UploadState } from "./transfer.ts";

/** The part size of the fake server, in bytes: small, so a short file is sent in several parts. */
export const PART = 4;

/** The file the tests send. */
export const delivery: Delivery = { issue: "WEB-1", comment: null, name: "a.txt", size: 10 };

/** The attachment the fake server makes of a finished upload. */
export const done: Attachment = {
  added_at: "2026-01-01T00:00:00Z",
  added_by: { shown: "someone" },
  download_url: "/files/x",
  id: "att",
  issue: "WEB-1",
  may_delete: true,
  media_type: "text/plain",
  name: "a.txt",
  preview: "without",
  size: 10,
  version: 1,
};

/** What the server says when it refuses with this code. */
export function problem(code: ErrorCode, status: number): Problem {
  return {
    type: code,
    title: "x",
    status,
    code,
    message_key: "error.server.unavailable",
    params: {},
  };
}

/** The error of a connection that was lost: no answer came at all. */
export function lostConnection(): ApiError {
  return new ApiError(0, undefined);
}

/** A course of a transfer that does not wait, and notes what it was told. */
export function course(): Course & {
  readonly waited: number[];
  readonly seen: number[];
  readonly opened: string[];
} {
  const waited: number[] = [];
  const seen: number[] = [];
  const opened: string[] = [];
  return {
    waited,
    seen,
    opened,
    wait: (ms) => {
      waited.push(ms);
      return Promise.resolve();
    },
    progress: (sent) => {
      seen.push(sent);
    },
    started: (upload) => {
      opened.push(upload);
    },
  };
}

/** What the fake server did, for a test to look at. */
export interface FakeUploads {
  readonly transport: Transport;
  /** The offsets of the parts that arrived, in order. */
  readonly sent: number[];
  /** How many uploads were started. */
  readonly started: () => number;
  /** The uploads that were thrown away. */
  readonly cancelled: string[];
  /** The uploads the server has, with the bytes of each. */
  readonly held: Map<string, number>;
}

/**
 * A server that keeps the uploads it was given and takes parts of [`PART`] bytes. `failures` are
 * what the first sends answer with, one each (`undefined` lets that send through).
 */
export function fakeUploads(
  failures: readonly (Readonly<ApiError> | undefined)[] = [],
): FakeUploads {
  const held = new Map<string, number>();
  const sent: number[] = [];
  const cancelled: string[] = [];
  let tries = 0;
  let count = 0;
  const view = (id: string): UploadState => ({
    id,
    offset: held.get(id) ?? 0,
    part_bytes: PART,
    size: delivery.size,
  });
  const known = (id: string): Promise<UploadState> =>
    held.has(id) ? Promise.resolve(view(id)) : Promise.reject(new ApiError(404, undefined));
  const transport: Transport = {
    start: () => {
      count += 1;
      held.set(`u${String(count)}`, 0);
      return Promise.resolve(view(`u${String(count)}`));
    },
    state: known,
    send: (id, offset, bytes) => {
      const failure = failures[tries];
      tries += 1;
      if (failure !== undefined) {
        return Promise.reject(failure);
      }
      return known(id).then(() => {
        sent.push(offset);
        held.set(id, offset + bytes.size);
        return view(id);
      });
    },
    finish: () => Promise.resolve(done),
    cancel: (id) => {
      cancelled.push(id);
      held.delete(id);
      return Promise.resolve();
    },
  };
  return { transport, sent, started: () => count, cancelled, held };
}
