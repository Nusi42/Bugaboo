import { ApiError, dataOf } from "@bugaboo/api-client";
import { setTimeout as sleep } from "node:timers/promises";

import { giveRole, roleName } from "../../app/e2e/support/access.ts";
import { ada, createPerson, ownId } from "../../app/e2e/support/accounts.ts";
import { type ApiSession } from "../../app/e2e/support/api.ts";
import { backUp } from "../../app/e2e/support/operations-scene.ts";
import { removeByApi, switchOffByApi } from "../../app/e2e/support/privacy.ts";
import { type Instance } from "../../app/e2e/support/server.ts";
import { type BookWriter, type Work } from "./book.ts";

// While the people work (architecture §11, "The long run"; PRIV-04, OPS-07): an administrator
// switches a person off whose page is open, makes a backup through the route at the middle of the
// run, and switches off and removes a person who wrote a comment on a shared issue. The book says
// what happened; `bb-load-check` judges it, and the copy the backup is restored into.

/** The project the people work in. */
const PROJECT = "LOAD";

/** The people who leave during a run, each signed in. */
interface Leavers {
  /** Has a page of an issue open when they are switched off. */
  readonly off: ApiSession;
  /** Wrote a comment on a shared issue; is switched off and removed while the others work. */
  readonly gone: ApiSession;
}

/** What the steps need: the administrator, the book, and when the run started and ends. */
interface Steps {
  readonly admin: ApiSession;
  readonly book: BookWriter;
  readonly started: number;
  readonly deadline: number;
}

/** A Team member of the project who leaves later. */
async function leaver(
  server: Instance,
  admin: ApiSession,
  name: string,
  email: string,
): Promise<ApiSession> {
  const session = await createPerson(server, admin, { name, email, password: ada.password });
  await giveRole(admin, await ownId(session), roleName.teamMember, PROJECT);
  return session;
}

/** The people who leave; the one who is removed first writes a comment on a shared issue. */
async function prepareLeavers(
  server: Instance,
  admin: ApiSession,
  issue: Work,
  book: BookWriter,
): Promise<Leavers> {
  const off = await leaver(server, admin, "Load Switched Off", "off@example.test");
  const gone = await leaver(server, admin, "Load Removed", "gone@example.test");
  dataOf(
    await gone.client.POST("/api/v1/issues/{issue}/comments", {
      params: { path: { issue: issue.key } },
      body: { text: "Handing this over before I leave." },
    }),
  );
  book.comment(issue.key);
  return { off, gone };
}

/** Waits until the share of the working time of the run has passed. */
async function until(steps: Steps, share: number): Promise<void> {
  const at = steps.started + (steps.deadline - steps.started) * share;
  await sleep(Math.max(0, at - Date.now()));
}

/** Whether a request was refused because the session is over: the person may not sign in. */
function signedOut(error: unknown): boolean {
  return error instanceof ApiError && error.status === 401;
}

/**
 * A person reads an issue (their page is open), is switched off, and saves a new title on what
 * they read: the save must be refused.
 */
async function switchOffWhileThePageIsOpen(
  steps: Steps,
  person: ApiSession,
  work: Work,
): Promise<void> {
  const read = dataOf(
    await person.client.GET("/api/v1/issues/{issue}", { params: { path: { issue: work.key } } }),
  );
  const id = await ownId(person);
  await switchOffByApi(steps.admin, id);
  let refused = false;
  try {
    const title = "Saved after I was switched off";
    const kept = dataOf(
      await person.client.PATCH("/api/v1/issues/{issue}", {
        params: { path: { issue: work.key } },
        body: { version: read.issue.version ?? 0, title },
      }),
    );
    // Wrongly kept: the book says so, so the versions of the issue still add up.
    steps.book.kept({ key: work.key, version: kept.version ?? 0, title });
  } catch (error) {
    refused = signedOut(error);
  }
  steps.book.left({ person: id, removed: false, refused_at_next_save: refused });
}

/** A person who wrote a comment is switched off, refused at their next comment, and removed. */
async function removeWhileTheyWork(steps: Steps, person: ApiSession, work: Work): Promise<void> {
  const id = await ownId(person);
  await switchOffByApi(steps.admin, id);
  let refused = false;
  try {
    dataOf(
      await person.client.POST("/api/v1/issues/{issue}/comments", {
        params: { path: { issue: work.key } },
        body: { text: "Written after I was switched off" },
      }),
    );
    steps.book.comment(work.key);
  } catch (error) {
    refused = signedOut(error);
  }
  await removeByApi(steps.admin, id);
  steps.book.left({ person: id, removed: true, refused_at_next_save: refused });
}

/** A backup through the route, as *Make a backup now* asks for one; the book notes what came before. */
async function backUpWhileTheyWork(steps: Steps): Promise<void> {
  const before = steps.book.counts().kept;
  steps.book.backedUp({ name: await backUp(steps.admin), kept_before: before });
}

/**
 * What happens while the people work: at a quarter of the time a person is switched off with a
 * page open, at the middle the backup is made, and at three quarters a person is removed.
 */
async function leaveWhileTheyWork(steps: Steps, leavers: Leavers, work: Work): Promise<void> {
  await until(steps, 1 / 4);
  await switchOffWhileThePageIsOpen(steps, leavers.off, work);
  await until(steps, 1 / 2);
  await backUpWhileTheyWork(steps);
  await until(steps, 3 / 4);
  await removeWhileTheyWork(steps, leavers.gone, work);
  await Promise.all([leavers.off.dispose(), leavers.gone.dispose()]);
}

/**
 * Prepares the people who leave before the run starts, on the first shared issue; what happens
 * while the people work, from the moment they start until the run ends.
 */
export async function prepareLeaving(
  server: Instance,
  admin: ApiSession,
  issues: readonly Work[],
  book: BookWriter,
): Promise<(started: number, deadline: number) => Promise<void>> {
  const [work] = issues;
  if (work === undefined) {
    throw new TypeError("A run needs an issue.");
  }
  const leavers = await prepareLeavers(server, admin, work, book);
  return (started, deadline) =>
    leaveWhileTheyWork({ admin, book, started, deadline }, leavers, work);
}
