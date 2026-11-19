import { dataOf } from "@bugaboo/api-client";

import { type ApiSession } from "../../app/e2e/support/api.ts";
import { type Reading } from "./judge.ts";
import { type KnownRecords } from "./known.ts";
import {
  type EntryRow,
  type IssueRow,
  type IssuesRead,
  knownRead,
  workDayOf,
} from "./read-known.ts";

// Reads through the API what the upgrade test compares (architecture §11, "The upgrade test"), as
// an administrator of the sample, who sees every project, and the work days as the person with the
// role *HR*. The reads of the issues, the people and the weeks go side by side.

type Client = ApiSession["client"];

/** As many items as a page of the API gives. */
const PAGE = 100;
/** The days the working hours are read for, from the end of the sample: more than it has. */
const DAYS_READ = 126;
const MS_PER_DAY = 86_400_000;
const DAYS_PER_WEEK = 7;

/** A page of a paged route. */
interface Page<Item> {
  readonly items: readonly Item[];
  readonly next_cursor: string | null;
}

/** Every item of a paged route, from the page at `cursor` on. */
async function everything<Item>(
  page: (cursor: string | undefined) => Promise<Page<Item>>,
  cursor?: string,
): Promise<readonly Item[]> {
  const read = await page(cursor);
  return read.next_cursor === null
    ? read.items
    : [...read.items, ...(await everything(page, read.next_cursor))];
}

/** The cursor of the next page, as a query takes it. */
function after(cursor: string | undefined): { readonly cursor?: string } {
  return cursor === undefined ? {} : { cursor };
}

/** Every issue. */
async function issueRows(client: Client): Promise<readonly IssueRow[]> {
  const issues = await everything(async (cursor) =>
    dataOf(
      await client.GET("/api/v1/search/issues", {
        params: { query: { fields: "readable_id,title,status", limit: PAGE, ...after(cursor) } },
      }),
    ),
  );
  const rows: IssueRow[] = [];
  for (const issue of issues) {
    rows.push({
      key: issue.readable_id ?? "",
      title: issue.title ?? "",
      status: issue.status?.key ?? "",
    });
  }
  return rows;
}

/** The texts of the comments of one issue. */
async function commentsOn(client: Client, issue: string): Promise<readonly string[]> {
  const history = await everything(async (cursor) =>
    dataOf(
      await client.GET("/api/v1/issues/{issue}/history", {
        params: { path: { issue }, query: { tab: "comments", limit: PAGE, ...after(cursor) } },
      }),
    ),
  );
  const texts: string[] = [];
  for (const entry of history) {
    if (entry.kind === "comment") {
      texts.push(entry.text ?? "");
    }
  }
  return texts;
}

/** The time entries of one issue. */
async function timeEntriesOn(client: Client, issue: string): Promise<readonly EntryRow[]> {
  const entries = await everything(async (cursor) =>
    dataOf(
      await client.GET("/api/v1/issues/{issue}/time-entries", {
        params: { path: { issue }, query: { limit: PAGE, ...after(cursor) } },
      }),
    ),
  );
  const rows: EntryRow[] = [];
  for (const entry of entries) {
    rows.push({ date: entry.date, minutes: entry.minutes ?? 0 });
  }
  return rows;
}

/**
 * How many reads go to the server at once. Hundreds at once (one for each issue) leave the
 * computer without free connections, and the reads fail with no answer.
 */
const AT_ONCE = 8;

/** What `read` gives for each of `things`, in their order, read [`AT_ONCE`] side by side. */
async function eachOf<Thing, Read>(
  things: readonly Thing[],
  read: (thing: Thing) => Promise<Read>,
): Promise<readonly Read[]> {
  if (things.length === 0) {
    return [];
  }
  const first = await Promise.all(things.slice(0, AT_ONCE).map((thing) => read(thing)));
  return [...first, ...(await eachOf(things.slice(AT_ONCE), read))];
}

/** The sum of the counts that `count` reads for each of `things`, read side by side. */
async function sumOf<Thing>(
  things: readonly Thing[],
  count: (thing: Thing) => Promise<number>,
): Promise<number> {
  const counts = await eachOf(things, count);
  return counts.reduce((sum, one) => sum + one, 0);
}

/** How many items all the lists have together. */
function totalOf(lists: readonly (readonly unknown[])[]): number {
  return lists.reduce((sum, list) => sum + list.length, 0);
}

/** The day of `moment` (milliseconds since 1970) as the API takes it. */
function dayOf(moment: number): string {
  return new Date(moment).toISOString().slice(0, "YYYY-MM-DD".length);
}

/** Somebody of the sample: ID and name. */
interface Somebody {
  readonly id: string;
  readonly name: string;
}

/** The people of the organization. */
async function people(client: Client): Promise<readonly Somebody[]> {
  const found = await everything(async (cursor) =>
    dataOf(
      await client.GET("/api/v1/people", {
        params: { query: { fields: "person", limit: PAGE, ...after(cursor) } },
      }),
    ),
  );
  const everybody: Somebody[] = [];
  for (const item of found) {
    if (item.person !== undefined && item.person !== null && "id" in item.person) {
      everybody.push({ id: item.person.id, name: item.person.name });
    }
  }
  return everybody;
}

/** The first moment of each week of the days read, back from the end of the sample. */
function weeksBefore(end: number): readonly number[] {
  return Array.from(
    { length: DAYS_READ / DAYS_PER_WEEK },
    (_, week) => end - (DAYS_READ - week * DAYS_PER_WEEK) * MS_PER_DAY,
  );
}

/** How many days one person worked in the week from `first`. */
async function daysWorked(client: Client, person: string, first: number): Promise<number> {
  const last = first + (DAYS_PER_WEEK - 1) * MS_PER_DAY;
  const read = dataOf(
    await client.GET("/api/v1/people/{person}/work-days", {
      params: { path: { person }, query: { from: dayOf(first), to: dayOf(last) } },
    }),
  );
  let days = 0;
  for (const day of read.days) {
    days += day.periods.length > 0 ? 1 : 0;
  }
  return days;
}

/** Who reads: an administrator, and the person with the role *HR*, who sees everybody's work days. */
export interface Readers {
  readonly administrator: Client;
  readonly hr: Client;
}

/** The names of the agents, sorted. */
async function agentNames(client: Client): Promise<readonly string[]> {
  const agents = await everything(async (cursor) =>
    dataOf(
      await client.GET("/api/v1/organization/agents", {
        params: { query: { limit: PAGE, ...after(cursor) } },
      }),
    ),
  );
  const names: string[] = [];
  for (const agent of agents) {
    names.push(agent.name ?? "");
  }
  return names.toSorted();
}

/** The backup the upgrade made, by its record, and whether it is among the files. */
async function backupOfUpgrade(
  client: Client,
): Promise<Pick<Reading, "backupBeforeUpgrade" | "backupFileThere">> {
  const backups = dataOf(await client.GET("/api/v1/admin/backups", {}));
  let made: string | undefined;
  for (const run of backups.runs) {
    made = run.maker === "upgrade" ? run.name : made;
  }
  let fileThere = false;
  for (const file of backups.files) {
    fileThere ||= file.name === made;
  }
  return { backupBeforeUpgrade: made, backupFileThere: fileThere };
}

/** How many away times the person of the goal has around the end of the sample. */
async function awayTimesOf(client: Client, person: string, end: number): Promise<number> {
  const times = dataOf(
    await client.GET("/api/v1/people/{person}/work-times", {
      params: {
        path: { person },
        query: {
          from: dayOf(end - DAYS_READ * MS_PER_DAY),
          to: dayOf(end + DAYS_READ * MS_PER_DAY),
        },
      },
    }),
  );
  return times.away.length;
}

/** The issues, and the comments and the time entries of each, read side by side. */
async function issuesRead(client: Client): Promise<IssuesRead> {
  const rows = await issueRows(client);
  const counted = dataOf(await client.GET("/api/v1/search/issues/count", {}));
  if (counted.issues !== rows.length) {
    throw new Error(
      `The search counts ${String(counted.issues)} issues and lists ${String(rows.length)}.`,
    );
  }
  return {
    rows,
    comments: await eachOf(rows, (row) => commentsOn(client, row.key)),
    entries: await eachOf(rows, (row) => timeEntriesOn(client, row.key)),
  };
}

/**
 * What the upgrade test compares, read through the API; `end` is the end of the sample, and
 * `known` the known records of the dump, whose keys are looked up.
 */
export async function readAfterUpgrade(
  { administrator: client, hr }: Readers,
  end: number,
  goalPerson: string,
  known: KnownRecords,
): Promise<Reading> {
  const issues = await issuesRead(client);
  const everybody = await people(client);
  const named = (name: string): Somebody | undefined =>
    everybody.find((person) => person.name === name);
  const goal = named(goalPerson);
  if (goal === undefined) {
    throw new Error(`${goalPerson} of the sample is not among the people.`);
  }
  return {
    issues: issues.rows.length,
    comments: totalOf(issues.comments),
    timeEntries: totalOf(issues.entries),
    workDays: await sumOf(everybody, (person) =>
      sumOf(weeksBefore(end), (week) => daysWorked(hr, person.id, week)),
    ),
    awayTimes: await awayTimesOf(client, goal.id, end),
    ownThemes: dataOf(await client.GET("/api/v1/themes", {})).themes.length,
    agents: await agentNames(client),
    known: await knownRead(issues, known, (day) => workDayOf(hr, named(day.person)?.id, day.date)),
    ...(await backupOfUpgrade(client)),
  };
}
