import {
  conflictOf,
  dataOf,
  descriptionEditPart,
  editAnyway,
  heartbeatSeconds,
  keepsTextAsDraft,
  newEditorId,
  readDraft,
  letGoOfEditLock,
  sendHeartbeat,
  takeEditLock,
  type EditTarget,
} from "@bugaboo/api-client";

import { type Work } from "./book.ts";
import { broke, codeOf, type Context, read, tag } from "./context.ts";
import { pick } from "./random.ts";
import { searchNow } from "./search.ts";
import { addFile, hear, moveOn } from "./work.ts";

/** How often a stale change is sent again on the newer version before it is called stuck. */
const MOST_TRIES = 40;

function whatOf(work: Work): EditTarget {
  return { record: { kind: "issue", id: work.id }, part: descriptionEditPart };
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
    context.book.kept({ key: work.key, version: changed.version ?? 0, title });
  } catch (error) {
    const newer = conflictOf(error);
    if (newer === undefined) {
      throw error;
    }
    context.book.conflict();
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
  context.book.comment(work.key);
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
  if (asked.kind === "taken") {
    return asked.lock_token;
  }
  if (!asked.can_edit_anyway || pick(context.random, 4) !== 0) {
    return undefined;
  }
  const over = await editAnyway(context.client, whatOf(work), editor);
  context.book.tookOver();
  return over.kind === "taken" ? over.lock_token : undefined;
}

/** A refused save keeps the text: whether it is the draft of the person is written in the book. */
async function keptAsDraft(context: Context, work: Work, text: string): Promise<void> {
  const draft = await readDraft(context.client, {
    record: { kind: "issue", id: work.id },
    part: descriptionEditPart,
  });
  context.book.drafted({
    holder: context.holder,
    key: work.key,
    text,
    ending: "refused",
    found_as_draft: draft?.text === text,
  });
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
    // The editor goes away without letting go: the lease has to end the lock, the draft stays.
    context.book.drafted({ holder: context.holder, key: work.key, text, ending: "vanished" });
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
    context.book.kept({ key: work.key, version: saved.version ?? 0, text });
    context.book.drafted({ holder: context.holder, key: work.key, text, ending: "saved" });
  } catch (error) {
    const code = codeOf(error);
    if (!keepsTextAsDraft(code)) {
      throw error;
    }
    await keptAsDraft(context, work, text);
    if (code === "edit.version-changed") {
      // The lock is still the person's: the page lets it go, as the browser does.
      await letGoOfEditLock(context.client, whatOf(work), token);
    }
  }
}

async function describe(context: Context, work: Work): Promise<void> {
  const token = await lockOn(context, work, newEditorId());
  if (token !== undefined) {
    await save(context, work, token, await read(context, work));
  }
}

/** What a Guest does on the issue they reported: comment, add a file, look, read their notifications. */
async function guestDoes(context: Context, work: Work, kind: number): Promise<void> {
  if (kind < 30) {
    await comment(context, work);
  } else if (kind < 55) {
    await addFile(context, work);
  } else if (kind < 80) {
    await hear(context);
  } else if (kind < 90) {
    await searchNow(context);
  } else {
    await look(context, work);
  }
}

/** What a person does next: changing, writing and editing, moving issues along their workflow, adding files, and looking in between. */
async function doNext(context: Context, work: Work, kind: number): Promise<void> {
  if (context.guest) {
    await guestDoes(context, work, kind);
  } else if (kind < 20) {
    await change(context, work);
  } else if (kind < 40 && Date.now() < context.editUntil) {
    await describe(context, work);
  } else if (kind < 55) {
    await comment(context, work);
  } else if (kind < 75) {
    await moveOn(context, work);
  } else if (kind < 80) {
    await addFile(context, work);
    await hear(context);
  } else if (kind < 90) {
    await searchNow(context);
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
