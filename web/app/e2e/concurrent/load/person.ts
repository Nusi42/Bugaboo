import {
  ApiError,
  conflictOf,
  dataOf,
  editAnyway,
  heartbeatSeconds,
  newEditorId,
  readDraft,
  readEditLock,
  releaseEditLock,
  sendHeartbeat,
  takeEditLock,
  type Client,
  type EditWhat,
} from "@bugaboo/api-client";

import { type Book, type Work } from "./book.ts";
import { pick } from "./random.ts";

/** One person at work: who they are, what they work on, and when to stop. */
export interface Context {
  readonly number: number;
  /** The ID of the person. */
  readonly holder: string;
  readonly client: Client;
  readonly book: Book;
  readonly issues: readonly Work[];
  readonly random: () => number;
  /** When to stop, as milliseconds since 1970. */
  readonly deadline: number;
  /**
   * Until when people edit under locks. After it only short values and comments change, so every
   * lease has ended when the run does and a lock that is still held then is one that outlived it.
   */
  readonly editUntil: number;
}

/** The refusals of a save that keep the text as a draft. */
const KEPT_AS_DRAFT: ReadonlySet<string> = new Set([
  "edit.lock_lost",
  "edit.taken_over",
  "edit.version_changed",
]);

/** How often a stale change is sent again on the newer version before it is called stuck. */
const MOST_TRIES = 40;

function whatOf(work: Work): EditWhat {
  return { record: { kind: "issue", id: work.id }, part: "description" };
}

/** The code of a refusal, or nothing for a failure that is not one. */
function codeOf(error: unknown): string | undefined {
  return error instanceof ApiError ? error.problem?.code : undefined;
}

/** Writes down what broke a rule, with who was doing what. */
export function broke(context: Context, doing: string, error: unknown): void {
  const said = error instanceof Error ? error.message : String(error);
  context.book.note({
    kind: "broken",
    message: `person ${String(context.number)} ${doing}: ${said}`,
  });
}

/** A number for a title or a text, so that no two are the same. */
function tag(context: Context): string {
  return `${String(context.number)}-${String(pick(context.random, 1_000_000))}`;
}

/** Reads the issue and writes down the version that was read. */
async function read(context: Context, work: Work): Promise<number> {
  const found = dataOf(
    await context.client.GET("/api/v1/issues/{issue}", { params: { path: { issue: work.key } } }),
  );
  const version = found.issue.version ?? 0;
  context.book.note({ kind: "seen", person: context.number, key: work.key, version });
  return version;
}

/** Changes the title on the version that was read; a change that came second is sent again (*Use mine*). */
async function changeOn(
  context: Context,
  work: Work,
  title: string,
  version: number,
  tries: number,
): Promise<void> {
  if (tries >= MOST_TRIES) {
    broke(context, "changed a title", new Error(`stuck on ${work.key}`));
    return;
  }
  try {
    const changed = dataOf(
      await context.client.PATCH("/api/v1/issues/{issue}", {
        params: { path: { issue: work.key } },
        body: { version, title },
      }),
    );
    context.book.note({ kind: "kept", key: work.key, version: changed.version ?? 0 });
  } catch (error) {
    const newer = conflictOf(error);
    if (newer === undefined) {
      throw error;
    }
    context.book.note({ kind: "conflict" });
    await changeOn(context, work, title, newer.version ?? version, tries + 1);
  }
}

async function change(context: Context, work: Work): Promise<void> {
  await changeOn(context, work, `Title ${tag(context)}`, await read(context, work), 0);
}

async function comment(context: Context, work: Work): Promise<void> {
  dataOf(
    await context.client.POST("/api/v1/issues/{issue}/comments", {
      params: { path: { issue: work.key } },
      body: { text: `Comment ${tag(context)}` },
    }),
  );
  context.book.note({ kind: "comment", key: work.key });
}

/** Looks around: the issue and its history. */
async function look(context: Context, work: Work): Promise<void> {
  await read(context, work);
  dataOf(
    await context.client.GET("/api/v1/issues/{issue}/history", {
      params: { path: { issue: work.key }, query: { limit: 20 } },
    }),
  );
}

/** The lock, taken, or taken over from somebody when the person may; nothing when somebody holds it. */
async function lockOn(context: Context, work: Work, editor: string): Promise<string | undefined> {
  const asked = await takeEditLock(context.client, whatOf(work), editor);
  if (asked.status === "taken") {
    return asked.lock_token;
  }
  if (!asked.can_edit_anyway || pick(context.random, 4) !== 0) {
    return undefined;
  }
  const over = await editAnyway(context.client, whatOf(work), editor);
  context.book.note({ kind: "took-over" });
  return over.status === "taken" ? over.lock_token : undefined;
}

/** Writes down that the editor went away: the lock must end by itself within the lease. */
async function vanish(context: Context, work: Work): Promise<void> {
  const held = await readEditLock(context.client, whatOf(work));
  if (held?.yours_elsewhere === true) {
    context.book.note({
      kind: "abandoned",
      work,
      holder: context.holder,
      since: held.since,
      at: Date.now(),
    });
  }
}

/** A refused save keeps the text: it has to be the draft of the person. */
async function keptAsDraft(
  context: Context,
  work: Work,
  text: string,
  code: string,
): Promise<void> {
  const draft = await readDraft(context.client, {
    kind: "issue",
    id: work.id,
    part: "description",
  });
  if (draft?.text === text) {
    context.book.note({ kind: "refused", code });
  } else {
    context.book.note({
      kind: "broken",
      message: `the text of a refused save is not the draft: ${text}`,
    });
  }
}

function pause(milliseconds: number): Promise<void> {
  return new Promise((done) => {
    setTimeout(done, milliseconds);
  });
}

/** Saves the description under the lock, or finds the text kept as the draft. */
async function save(context: Context, work: Work, token: string, version: number): Promise<void> {
  const text = `Text ${tag(context)} on ${work.key}`;
  await sendHeartbeat(context.client, whatOf(work), token, { text, base_version: version });
  if (pick(context.random, 6) === 0) {
    await vanish(context, work);
    return;
  }
  // The person types for a moment, in which others read and change the issue.
  await pause(pick(context.random, heartbeatSeconds * 20));
  try {
    const saved = dataOf(
      await context.client.PUT("/api/v1/issues/{issue}/description", {
        params: { path: { issue: work.key } },
        body: { text, version, lock_token: token },
      }),
    );
    context.book.note({ kind: "kept", key: work.key, version: saved.version ?? 0, text });
  } catch (error) {
    const code = codeOf(error);
    if (code === undefined || !KEPT_AS_DRAFT.has(code)) {
      throw error;
    }
    await keptAsDraft(context, work, text, code);
    if (code === "edit.version_changed") {
      // The lock is still the person's: the page lets it go, as the browser does.
      await releaseEditLock(context.client, whatOf(work), token);
    }
  }
}

async function describe(context: Context, work: Work): Promise<void> {
  const token = await lockOn(context, work, newEditorId());
  if (token !== undefined) {
    await save(context, work, token, await read(context, work));
  }
}

/** What a person does next: mostly changing, writing and editing, and looking in between. */
async function doNext(context: Context, work: Work, kind: number): Promise<void> {
  if (kind < 30) {
    await change(context, work);
  } else if (kind < 55 && Date.now() < context.editUntil) {
    await describe(context, work);
  } else if (kind < 70) {
    await comment(context, work);
  } else {
    await look(context, work);
  }
}

/** One thing a person does; what a rule forbids is written down, not thrown. */
async function act(context: Context): Promise<void> {
  const work = context.issues[pick(context.random, context.issues.length)];
  const kind = pick(context.random, 100);
  if (work === undefined) {
    return;
  }
  try {
    await doNext(context, work, kind);
  } catch (error) {
    broke(context, `did ${String(kind)} on ${work.key}`, error);
  }
}

/** Works until the time is over: one thing after the other, as a person does. */
export async function keepWorking(context: Context): Promise<void> {
  if (Date.now() < context.deadline) {
    await act(context);
    await keepWorking(context);
  }
}
