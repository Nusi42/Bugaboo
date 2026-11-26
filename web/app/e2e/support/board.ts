import { dataOf, type Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { expect, type Locator, type Page } from "@playwright/test";

import { savedSearchAddress } from "../../src/shared/paths.ts";
import { ada, signInThroughApi, type ApiSession, type Instance } from "./accounts.ts";
import { firstIssueType, issueOf, prepareWeb, setStatusAndPriority, versionOf } from "./issues.ts";
import { partIn } from "./parts.ts";

// The scene of the tests of the board (PLAN-06, UX §4.6): the project WEB of the tests of issues, its
// issues spread over the columns of the workflow *Simple*, and the saved search *Plan board*, a board
// of the project that Ada made, with a card limit on *In progress*.

/** The saved search that shows WEB as a board. */
export const boardName = "Plan board";

/**
 * The issues of the scene by what they are for. WEB-1 to WEB-3 are the issues of `prepareWeb`, Tasks, which
 * follow the workflow *Simple* (any status to any other); the rest are Bugs, whose workflow *Bug* leads from
 * *Open* only to *In progress* and *Won't fix*.
 */
export const cards = {
  open: "WEB-1",
  working: "WEB-2",
  other: "WEB-3",
  /** A Bug in *Open*: it cannot go to *Done* straight. */
  bug: "WEB-4",
  critical: "WEB-5",
  done: "WEB-6",
  /** In progress with the others, so the limit of two is passed. */
  third: "WEB-7",
} as const;

/** The card limit of *In progress* that the scene sets: the column holds three cards, so it is over. */
export const progressLimit = 2;

/**
 * The names of the statuses of the columns: those of *Bug* (Open, In progress, Testing, Done, Won't fix), whose
 * Bugs do not go from *Open* to *Done* straight. The Tasks of the scene follow *Simple*, which has no *Testing*.
 */
export const statusNames = {
  open: "Open",
  progress: "In progress",
  testing: "Testing",
  done: "Done",
} as const;

/** The ID of the saved search of the scene, after `prepareBoard`. */
let boardId = "";

/** Puts an issue into a status, on the version it has. */
export async function moveTo(
  admin: ApiSession,
  key: string,
  status: string,
  priority: Schema<"Priority"> = "normal",
): Promise<void> {
  const issue = await issueOf(admin, key);
  await setStatusAndPriority(admin, issue, await versionOf(admin, issue), { status, priority });
}

/** Makes an issue of the project WEB. */
async function made(admin: ApiSession, title: string): Promise<void> {
  dataOf(
    await admin.client.POST("/api/v1/projects/{project}/issues", {
      params: { path: { project: "WEB" } },
      body: { issue_type: await firstIssueType(admin), title },
    }),
  );
}

/** What the tests of a file add to the scene, so that `resetBoard` takes it away again. */
export const added = {
  issues: [] as string[],
  searches: [] as string[],
  shared: [] as string[],
  projects: 0,
};

/** Gets the server ready: WEB, three cards in *In progress*, one *Done*, three *Open*, and *Plan board*. */
export async function prepareBoard(server: Instance): Promise<void> {
  // A new server: what the tests of the file before added to another one is not here.
  added.issues.length = 0;
  added.searches.length = 0;
  added.shared.length = 0;
  added.projects = 0;
  await prepareWeb(server);
  const admin = await signInThroughApi(server, ada);
  await made(admin, "Fourth card");
  await made(admin, "Fifth card");
  await made(admin, "Sixth card");
  await made(admin, "Seventh card");
  await moveTo(admin, cards.working, "in-progress", "high");
  await moveTo(admin, cards.third, "in-progress");
  await moveTo(admin, cards.critical, "in-progress", "critical");
  await moveTo(admin, cards.done, "in-progress");
  await moveTo(admin, cards.done, "testing");
  await moveTo(admin, cards.done, "done");
  boardId = await saveBoard(admin, boardName, {
    columns: null,
    rows: null,
    card_limits: [{ column: statusNames.progress, limit: progressLimit }],
  });
  await admin.dispose();
}

/** Saves a board of the project WEB for the session's person. */
export async function saveBoard(
  session: ApiSession,
  name: string,
  options: Schema<"BoardOptionsBody">,
  text = "project is WEB",
): Promise<string> {
  return dataOf(
    await session.client.POST("/api/v1/saved-searches", {
      body: {
        name,
        search_text: text,
        presentation: {
          view: "board",
          columns: [],
          group_by: null,
          board: {
            columns: options.columns,
            rows: options.rows,
            card_limits: (options.card_limits ?? []).map(({ column, limit }) => ({
              column,
              limit,
            })),
          },
        },
      },
    }),
  ).id;
}

/** The ID of the saved search *Plan board*. */
export function plannedBoardId(): string {
  return boardId;
}

/** Opens *Plan board* and waits for its columns. */
export async function openBoard(page: Page): Promise<void> {
  await page.goto(savedSearchAddress(boardId));
  await expect(boardOn(page)).toBeVisible();
  await expect(cardOf(page, cards.open)).toBeVisible();
}

/** The board on the page. */
export function boardOn(page: Page): Locator {
  return partIn(page, "board");
}

/** A card, found by its Issue ID. */
export function cardOf(page: Page, key: string): Locator {
  return partIn(page, "board-card").filter({ hasText: key });
}

/** The list of the cards of a column (with a board without rows, the one cell of it). */
export function columnOf(page: Page, name: string): Locator {
  return page.getByRole("list", { name: t("board.cards-of", { column: name }), exact: true });
}

/** The head of a column: its name and how many cards it holds. */
export function headOf(page: Page, name: string): Locator {
  return partIn(page, "board-head.cell").filter({ hasText: name });
}

/** The part of a card a hand takes: its first line, which is no link. */
export function gripOf(page: Page, key: string): Locator {
  return partIn(cardOf(page, key), "board-card.line");
}

/** The menu button of a card, *Actions for WEB-4*. */
export function actionsOf(page: Page, key: string): Locator {
  return cardOf(page, key).getByRole("button", {
    name: t("common.actions-for", { name: key }),
    exact: true,
  });
}

/** Opens the menu of a card and chooses *Move to …*: the window of the columns it can go to. */
export async function openMoveTo(page: Page, key: string): Promise<Locator> {
  await actionsOf(page, key).click();
  await page.getByRole("button", { name: t("board.move-to"), exact: true }).click();
  const window = page.getByRole("dialog", { name: t("board.move-to-title", { key }) });
  await expect(window).toBeVisible();
  return window;
}

/** The status of an issue now, from the server. */
export async function statusOfIssue(session: ApiSession, key: string): Promise<string | undefined> {
  const issue = await issueOf(session, key);
  const read = dataOf(
    await session.client.GET("/api/v1/issues/{issue}", { params: { path: { issue: issue.key } } }),
  );
  return read.issue.status?.name;
}

/** The ID of the saved search of this name that the session's person made. */
export async function savedIdOf(session: ApiSession, name: string): Promise<string> {
  const list = dataOf(
    await session.client.GET("/api/v1/saved-searches", { params: { query: { scope: "mine" } } }),
  );
  for (const item of list.items) {
    if (item.name === name) {
      return item.id;
    }
  }
  throw new TypeError(`No saved search is called ${name}.`);
}

/** The ID of the project WEB. */
export async function webProjectId(session: ApiSession): Promise<string> {
  return dataOf(
    await session.client.GET("/api/v1/projects/{project}", {
      params: { path: { project: "WEB" } },
    }),
  ).project.id;
}

/** Shares a saved search with the project WEB, whose members then open it with their own rights. */
export async function shareWithWeb(session: ApiSession, search: string): Promise<void> {
  dataOf(
    await session.client.POST("/api/v1/saved-searches/{search}/shares", {
      params: { path: { search } },
      body: { kind: "project", id: await webProjectId(session) },
    }),
  );
}
