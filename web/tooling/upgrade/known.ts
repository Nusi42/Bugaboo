// The known records of the upgrade test (architecture §11, "The upgrade test", TEST-UPGRADE): a few
// records of the old sample, each found by its key, worked out from the events in the dump the old
// program wrote. The dump names every record by a label that counts the records of a kind in the
// order they first show up (`issue#3`), and the people in the order of its list of people, so a
// key is what a person reads: the title of an issue no other issue has, and the name of a person
// with a day. Pure, so its tests need no server.

/** An issue, by its title. */
export interface KnownIssue {
  readonly title: string;
  /** The key of its status. */
  readonly status: string;
}

/** The texts of the comments of one issue, sorted. */
export interface KnownComments {
  /** The title of the issue. */
  readonly issue: string;
  readonly texts: readonly string[];
}

/** The minutes of the time entries of one issue on one day, sorted. */
export interface KnownTimeEntries {
  /** The title of the issue. */
  readonly issue: string;
  readonly date: string;
  readonly minutes: readonly number[];
}

/** A stretch of a work day, in milliseconds since 1970. */
export interface KnownSpan {
  readonly start: number;
  readonly end: number;
}

/** One finished work day of one person. */
export interface KnownWorkDay {
  /** The name of the person. */
  readonly person: string;
  readonly date: string;
  readonly periods: readonly KnownSpan[];
}

/** The records the upgrade test finds by their keys and holds against the dump. */
export interface KnownRecords {
  /** The first and the last issue of the dump whose title no other issue has. */
  readonly issues: readonly KnownIssue[];
  readonly comments: KnownComments;
  readonly timeEntries: KnownTimeEntries;
  readonly workDay: KnownWorkDay;
}

/** One change of an event of the dump. */
interface Change {
  readonly field: string;
  readonly after: unknown;
}

/** An event of the dump: the label of its record and its changes. */
interface DumpEvent {
  readonly entity: string;
  readonly changes: readonly Change[];
}

/** One record of the dump: its label and the newest value of each field its events changed. */
interface DumpRecord {
  readonly label: string;
  readonly values: Readonly<Record<string, unknown>>;
}

/** The records of each kind, in the order they first show up. */
type Kinds = Readonly<Record<string, readonly DumpRecord[]>>;

function isObject(value: unknown): value is object {
  return typeof value === "object" && value !== null;
}

function textOf(value: unknown): string {
  return typeof value === "string" ? value : "";
}

/** The value of `key` of `value`, when it is an object. */
function fieldOf(value: unknown, key: string): unknown {
  return isObject(value) ? (Reflect.get(value, key) as unknown) : undefined;
}

/** The events of the dump. */
function eventsOf(dump: unknown): readonly DumpEvent[] {
  const events = fieldOf(dump, "events");
  if (!Array.isArray(events)) {
    throw new TypeError("The dump of the previous milestone has no events.");
  }
  return events.map((event: unknown) => {
    const changes = fieldOf(event, "changes");
    return {
      entity: textOf(fieldOf(event, "entity")),
      changes: (Array.isArray(changes) ? changes : []).map((change: unknown) => ({
        field: textOf(fieldOf(change, "field")),
        after: fieldOf(change, "after"),
      })),
    };
  });
}

/** Every record of every kind with the newest value of each field its events changed. */
function kindsOf(events: readonly DumpEvent[]): Kinds {
  const records = new Map<string, Record<string, unknown>>();
  for (const event of events) {
    const values = records.get(event.entity) ?? {};
    records.set(event.entity, values);
    for (const change of event.changes) {
      values[change.field] = change.after;
    }
  }
  const kinds: Record<string, DumpRecord[]> = {};
  for (const [label, values] of records) {
    const kind = label.slice(0, label.indexOf("#"));
    kinds[kind] = [...(kinds[kind] ?? []), { label, values }];
  }
  return kinds;
}

/** The first of `records` that `fits`, or a plain message that the dump has none. */
function first(
  records: readonly DumpRecord[],
  fits: (record: DumpRecord) => boolean,
  what: string,
): DumpRecord {
  const found = records.find((record) => fits(record));
  if (found === undefined) {
    throw new TypeError(`The dump of the previous milestone has no ${what}.`);
  }
  return found;
}

/** The milliseconds since 1970 of a moment of the dump or the API. */
export function momentOf(value: unknown): number {
  return Date.parse(textOf(value));
}

/** The stretches of a work day of the dump, when there are some and all of them are finished. */
function finishedPeriods(times: unknown): readonly KnownSpan[] | undefined {
  const periods = fieldOf(times, "periods");
  if (!Array.isArray(periods) || periods.length === 0) {
    return undefined;
  }
  const spans: KnownSpan[] = [];
  for (const period of periods as readonly unknown[]) {
    const end = fieldOf(period, "end");
    if (typeof end !== "string") {
      return undefined;
    }
    spans.push({ start: momentOf(fieldOf(period, "start")), end: momentOf(end) });
  }
  return spans;
}

/** The name of the person of the label `person#N`: the Nth of the dump's list of people. */
function personNamed(dump: unknown, label: unknown): string {
  const people = fieldOf(dump, "people");
  const number = Number(textOf(label).slice("person#".length));
  const name = fieldOf(Array.isArray(people) ? (people[number - 1] as unknown) : undefined, "name");
  if (typeof name !== "string") {
    throw new TypeError(`The dump of the previous milestone has no ${textOf(label)}.`);
  }
  return name;
}

/** The titles of the issues by label, and how many issues have each title. */
interface Titles {
  readonly byLabel: Readonly<Record<string, string>>;
  readonly counted: Readonly<Record<string, number>>;
}

function titlesOf(issues: readonly DumpRecord[]): Titles {
  const byLabel: Record<string, string> = {};
  const counted: Record<string, number> = {};
  for (const issue of issues) {
    const title = textOf(issue.values["title"]);
    byLabel[issue.label] = title;
    counted[title] = (counted[title] ?? 0) + 1;
  }
  return { byLabel, counted };
}

/** The title of the issue `label`. */
function titleOf(titles: Titles, label: unknown): string {
  return titles.byLabel[textOf(label)] ?? "";
}

/** Whether no other issue has the title of the issue `label`, so the title is its key. */
function keyed(titles: Titles, label: unknown): boolean {
  return titles.counted[titleOf(titles, label)] === 1;
}

/** The first and the last issue whose title no other issue has. */
function knownIssues(issues: readonly DumpRecord[], titles: Titles): readonly KnownIssue[] {
  const own = issues.filter((issue) => keyed(titles, issue.label));
  const what = "issue with a title of its own";
  return [first(own, () => true, what), first(own.toReversed(), () => true, what)].map(
    ({ values }) => ({ title: textOf(values["title"]), status: textOf(values["status"]) }),
  );
}

/** The comments of the first issue with a title of its own that has some. */
function knownComments(comments: readonly DumpRecord[], titles: Titles): KnownComments {
  const issue = first(comments, ({ values }) => keyed(titles, values["issue"]), "comment").values[
    "issue"
  ];
  return {
    issue: titleOf(titles, issue),
    texts: comments
      .filter(({ values }) => values["issue"] === issue)
      .map(({ values }) => textOf(values["text"]))
      .toSorted(),
  };
}

/** The time entries of the day of the first one on an issue with a title of its own. */
function knownTimeEntries(entries: readonly DumpRecord[], titles: Titles): KnownTimeEntries {
  const entry = first(entries, ({ values }) => keyed(titles, values["issue"]), "time entry").values;
  return {
    issue: titleOf(titles, entry["issue"]),
    date: textOf(entry["date"]),
    minutes: entries
      .filter(
        ({ values }) => values["issue"] === entry["issue"] && values["date"] === entry["date"],
      )
      .map(({ values }) => Number(values["minutes"]))
      .toSorted((a, b) => a - b),
  };
}

/** The first work day of the dump whose stretches are all finished. */
function knownWorkDay(dump: unknown, days: readonly DumpRecord[]): KnownWorkDay {
  const day = first(
    days,
    ({ values }) => finishedPeriods(values["times"]) !== undefined,
    "finished work day",
  ).values;
  return {
    person: personNamed(dump, day["person"]),
    date: textOf(day["date"]),
    periods: finishedPeriods(day["times"]) ?? [],
  };
}

/** The known records the dump of the old sample says. */
export function knownFrom(dump: unknown): KnownRecords {
  const kinds = kindsOf(eventsOf(dump));
  const issues = kinds["issue"] ?? [];
  const titles = titlesOf(issues);
  return {
    issues: knownIssues(issues, titles),
    comments: knownComments(kinds["comment"] ?? [], titles),
    timeEntries: knownTimeEntries(kinds["time_entry"] ?? [], titles),
    workDay: knownWorkDay(dump, kinds["work_day"] ?? []),
  };
}
