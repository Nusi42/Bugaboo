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
 * What the transfer asks of the server (the four calls of an upload, ADR-0032 §2). The app's one
 * connection implements it (`connection.ts`), and the tests of this file play the server.
 */
export interface Transport {
  readonly start: (delivery: Delivery) => Promise<UploadState>;
  readonly state: (upload: string) => Promise<UploadState>;
  readonly send: (upload: string, offset: number, bytes: Blob) => Promise<UploadState>;
  readonly finish: (upload: string) => Promise<Attachment>;
}

/**
 * How long to wait before each new try after something went wrong on the way, in milliseconds.
 * After the last one the transfer gives up and says the connection was lost.
 */
export const waitsBetweenTries: readonly number[] = [1_000, 2_000, 4_000, 8_000, 15_000, 15_000];

/** The HTTP status of an answer from a server that was too slow. */
const TIMED_OUT = 408;
/** The first status of the answers that say the server failed. */
const SERVER_FAILED = 500;
/** The answer that says the disk of the server is full: the person is told, it is not retried. */
const NO_ROOM = 507;

/**
 * Whether trying again can help: no answer came at all (the connection was lost), the server was
 * too slow or failed. A refusal never changes by asking again.
 */
export function isTemporary(error: unknown): boolean {
  return (
    error instanceof ApiError &&
    (error.status === 0 ||
      error.status === TIMED_OUT ||
      (error.status >= SERVER_FAILED && error.status !== NO_ROOM))
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
  try {
    const next = await transport.send(state.id, from, file.slice(from, to));
    course.progress(next.offset);
    return await sendParts(transport, delivery, file, course, next, 0);
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
}

/**
 * Sends one file: starts the upload, sends it in parts that each say where they start, and
 * finishes it. After a lost connection the transfer asks the server where it stands and goes on from
 * there, so a file that was half sent is never sent from the beginning (ATT-01). A refusal of the
 * server (too big, a type that is not accepted, no right) rejects at once.
 */
export async function sendFile(
  transport: Transport,
  delivery: Delivery,
  file: Blob,
  course: Course,
): Promise<Attachment> {
  const started = await withTries(course, () => transport.start(delivery));
  course.progress(started.offset);
  const sent = await sendParts(transport, delivery, file, course, started, 0);
  return withTries(course, () => transport.finish(sent.id));
}
