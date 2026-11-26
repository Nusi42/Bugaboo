import { dataOf, type Schema } from "@bugaboo/api-client";

import { ada, signInThroughApi, type ApiSession, type Instance } from "./accounts.ts";
import {
  cards,
  added,
  moveTo,
  prepareBoard,
  plannedBoardId,
  progressLimit,
  saveBoard,
  shareWithWeb,
  statusNames,
  webProjectId,
} from "./board.ts";
import { createIssue, issueOf, versionOf } from "./issues.ts";
import { inOrder } from "./sequence.ts";
import { askForWhoWorks } from "./workflows.ts";

// What the tests of the board add to its scene, and the way to take it away again: a file of tests runs on
// one server, so each test starts from the scene as `prepareBoard` made it (and a test run again meets nothing).

/** Makes issues in WEB, which start in *Open*: a column longer than its first page. */
export async function manyOpenCards(session: ApiSession, count: number): Promise<void> {
  await inOrder(
    Array.from({ length: count }, (_, at) => async () => {
      const issue = await createIssue(session, "WEB", `Extra card ${String(at)}`);
      added.issues.push(issue.key);
    }),
  );
}

/** Saves another board of WEB that `resetBoard` throws away again. */
export async function saveExtraBoard(
  session: ApiSession,
  name: string,
  options: Schema<"BoardOptionsBody">,
  text?: string,
): Promise<string> {
  const id = await saveBoard(session, name, options, text);
  added.searches.push(id);
  return id;
}

/** Shares *Plan board* with WEB, which `resetBoard` undoes. */
export async function sharePlanBoard(session: ApiSession): Promise<string> {
  const id = plannedBoardId();
  await shareWithWeb(session, id);
  added.shared.push(id);
  return id;
}

/** A short name for a project of its own in each call, so that a test run again does not meet the project it made before. */
export function nextProjectName(): string {
  added.projects += 1;
  return `SUB${String(added.projects)}`;
}

/** Where each issue of the scene starts, so a test can put them back. */
const starts: readonly (readonly [string, string])[] = [
  [cards.open, "open"],
  [cards.working, "in-progress"],
  [cards.other, "open"],
  [cards.bug, "open"],
  [cards.critical, "in-progress"],
  [cards.done, "done"],
  [cards.third, "in-progress"],
];

/** The key of the status an issue is in now. */
async function statusKeyOf(session: ApiSession, key: string): Promise<string | undefined> {
  const read = dataOf(
    await session.client.GET("/api/v1/issues/{issue}", { params: { path: { issue: key } } }),
  );
  return read.issue.status?.key;
}

/** Puts an issue in a status again: straight where the workflow allows it, else by *Open*. */
async function putBack(admin: ApiSession, key: string, status: string): Promise<void> {
  if ((await statusKeyOf(admin, key)) === status) {
    return;
  }
  try {
    await moveTo(admin, key, status);
  } catch {
    // The workflow does not lead there straight (*Done* from *In progress* of a Bug): by *Open*.
    await moveTo(admin, key, "open");
    await moveTo(admin, key, status);
  }
}

/** The cards where the scene put them, and *Plan board* as it was made: what the tests before changed is undone. */
export async function resetBoard(server: Instance): Promise<void> {
  const admin = await signInThroughApi(server, ada);
  await takeAwayWhatWasAdded(admin);
  await inOrder(
    starts.map(
      ([key, status]) =>
        () =>
          putBack(admin, key, status),
    ),
  );
  const saved = dataOf(
    await admin.client.GET("/api/v1/saved-searches/{search}", {
      params: { path: { search: plannedBoardId() } },
    }),
  );
  dataOf(
    await admin.client.PATCH("/api/v1/saved-searches/{search}", {
      params: { path: { search: plannedBoardId() } },
      body: {
        version: saved.version,
        presentation: {
          view: "board",
          columns: [],
          group_by: null,
          board: {
            columns: null,
            rows: null,
            card_limits: [{ column: statusNames.progress, limit: progressLimit }],
          },
        },
      },
    }),
  );
  await admin.dispose();
}

/** Takes away what the tests added: the extra cards, the other boards, the share, the assignments. */
async function takeAwayWhatWasAdded(admin: ApiSession): Promise<void> {
  await inOrder(
    added.issues.splice(0).map((key) => async () => {
      await admin.client.DELETE("/api/v1/issues/{issue}", { params: { path: { issue: key } } });
    }),
  );
  await inOrder(
    added.searches.splice(0).map((id) => async () => {
      await admin.client.DELETE("/api/v1/saved-searches/{search}", {
        params: { path: { search: id } },
      });
    }),
  );
  const project = await webProjectId(admin);
  await inOrder(
    added.shared.splice(0).map((id) => async () => {
      await admin.client.DELETE("/api/v1/saved-searches/{search}/shares/{kind}/{id}", {
        params: { path: { search: id, kind: "project", id: project } },
      });
    }),
  );
  const working = await issueOf(admin, cards.working);
  await admin.client.PATCH("/api/v1/issues/{issue}", {
    params: { path: { issue: cards.working } },
    body: { version: await versionOf(admin, working), assigned_to: [] },
  });
}

/** The scene in which the status change from *Open* to *In progress* of the Tasks asks for who does the work. */
export async function prepareBoardThatAsks(server: Instance): Promise<void> {
  await prepareBoard(server);
  const admin = await signInThroughApi(server, ada);
  await askForWhoWorks(admin, "WEB");
  await admin.dispose();
}
