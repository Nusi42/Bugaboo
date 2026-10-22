import { ApiError, type Schema } from "@bugaboo/api-client";

/** An upload as the server tells it: where it stands, and how big a part may be. */
export type UploadState = Schema<"UploadItem">;
/** A file that was added to an issue or a comment. */
export type Attachment = Schema<"AttachmentItem">;

/** The file to send, and where it goes. */
export interface Delivery {
  /** The issue, by its readable ID. */
  readonly issue: string;
  /** The comment the file is for; `null` for a file of the issue itself. */
  readonly comment: string | null;
  readonly name: string;
  readonly size: number;
}

/**
 * What the transfer asks of the server (the calls of an upload, ADR-0032 §2). The app's one
 * connection implements it (`connection.ts`), and the tests of this file play the server.
 */
export interface Transport {
  readonly start: (delivery: Delivery) => Promise<UploadState>;
  readonly state: (upload: string) => Promise<UploadState>;
  readonly send: (upload: string, offset: number, bytes: Blob) => Promise<UploadState>;
  readonly finish: (upload: string) => Promise<Attachment>;
  /** Throws an unfinished upload away; the server does not mind one that is gone. */
  readonly cancel: (upload: string) => Promise<void>;
}

/**
 * How long to wait before each new try after something went wrong on the way, in milliseconds.
 * After the last one the transfer gives up and says the connection was lost.
 */
export const waitsBetweenTries: readonly number[] = [1_000, 2_000, 4_000, 8_000, 15_000, 15_000];

/** The HTTP status of an answer from a server that was too slow. */
const TIMED_OUT = 408;
/** The HTTP status of an answer that says the server does not know the thing asked for. */
const NOT_FOUND = 404;
/** The first status of the answers that say the server failed. */
const SERVER_FAILED = 500;

/** The answer that says the disk of the server is full: the person is told, it is not retried. */
function hasNoRoom(error: Readonly<ApiError>): boolean {
  return error.problem?.code === "file.no_room";
}

/**
 * Whether trying again can help: no answer came at all (the connection was lost), the server was
 * too slow or failed. A refusal never changes by asking again.
 */
export function isTemporary(error: unknown): boolean {
  return (
    error instanceof ApiError &&
    (error.status === 0 ||
      error.status === TIMED_OUT ||
      (error.status >= SERVER_FAILED && !hasNoRoom(error)))
  );
}

/** The refusal that says a part did not start where the server's copy ends. */
function isOutOfOrder(error: unknown): boolean {
  return error instanceof ApiError && error.problem?.code === "file.wrong_part";
}

/** What a transfer needs besides the server: a way to wait, and to tell how far it is. */
export interface Course {
  readonly wait: (milliseconds: number) => Promise<void>;
  /** Called with the number of bytes the server has, after each part. */
  readonly progress: (sent: number) => void;
  /**
   * Called with the ID of the upload as soon as it is there, so a transfer that is lost can go on
   * with it later instead of starting over, and a refusal can throw it away.
   */
  readonly started: (upload: string) => void;
}

/**
 * Runs `step` and, when it fails in a way that trying again can help, waits and runs it again,
 * once for each of the waits. Anything else fails at once, and so does the failure after the last
 * wait.
 */
async function withTries<T>(
  course: Course,
  step: () => Promise<T>,
  waits: readonly number[] = waitsBetweenTries,
): Promise<T> {
  const [wait, ...rest] = waits;
  if (wait === undefined) {
    return step();
  }
  try {
    return await step();
  } catch (error) {
    if (!isTemporary(error)) {
      throw error;
    }
    await course.wait(wait);
    return withTries(course, step, rest);
  }
}

/** The state of an upload after a part that may or may not have arrived: ask the server. */
function whereItStands(transport: Transport, course: Course, upload: string): Promise<UploadState> {
  return withTries(course, () => transport.state(upload));
}

/**
 * Sends the parts from where the server stands to the end. A part that fails in a way that trying
 * again can help, or that does not start where the server's copy ends, is followed by the question
 * where the server stands, and the transfer goes on from there; `failures` counts the parts that
 * failed one after the other, and the transfer gives up after as many as there are waits.
 */
async function sendParts(
  transport: Transport,
  delivery: Delivery,
  file: Blob,
  course: Course,
  state: UploadState,
  failures: number,
): Promise<UploadState> {
  if (state.offset >= delivery.size) {
    return state;
  }
  const from = state.offset;
  const to = Math.min(delivery.size, from + state.part_bytes);
  let next: UploadState;
  try {
    next = await transport.send(state.id, from, file.slice(from, to));
  } catch (error) {
    const wait = waitsBetweenTries[failures];
    if (!(isTemporary(error) || isOutOfOrder(error)) || wait === undefined) {
      throw error;
    }
    if (isTemporary(error)) {
      await course.wait(wait);
    }
    const stands = await whereItStands(transport, course, state.id);
    course.progress(stands.offset);
    return sendParts(transport, delivery, file, course, stands, failures + 1);
  }
  // The next part is sent outside the `try` above: a part that fails for good down the line must
  // not be caught here and tried again by every part before it.
  course.progress(next.offset);
  return sendParts(transport, delivery, file, course, next, 0);
}

/** Whether the server says it does not know the upload (it is gone, or never was). */
function isGone(error: unknown): boolean {
  return error instanceof ApiError && error.status === NOT_FOUND;
}

/** The upload to send: the one to go on with if the server still has it, else a new one. */
async function beginOrResume(
  transport: Transport,
  delivery: Delivery,
  course: Course,
  resume: string | undefined,
): Promise<UploadState> {
  if (resume !== undefined) {
    try {
      return await whereItStands(transport, course, resume);
    } catch (error) {
      if (!isGone(error)) {
        throw error;
      }
    }
  }
  return withTries(course, () => transport.start(delivery));
}

/**
 * Sends one file: starts the upload, sends it in parts that each say where they start, and
 * finishes it. After a lost connection the transfer asks the server where it stands and goes on from
 * there, so a file that was half sent is never sent from the beginning (ATT-01). With the ID of an
 * upload of an earlier try (`resume`) it goes on with that one, from where the server has it. A
 * refusal of the server (too big, a type that is not accepted, no right) rejects at once.
 */
export async function sendFile(
  transport: Transport,
  delivery: Delivery,
  file: Blob,
  course: Course,
  resume?: string,
): Promise<Attachment> {
  const started = await beginOrResume(transport, delivery, course, resume);
  course.started(started.id);
  course.progress(started.offset);
  const sent = await sendParts(transport, delivery, file, course, started, 0);
  return withTries(course, () => transport.finish(sent.id));
}
