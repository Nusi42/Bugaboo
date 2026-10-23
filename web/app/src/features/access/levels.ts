import type { Schema } from "@bugaboo/api-client";

/** The fields of the catalogue, the kinds of people information and their levels, as the API words them. */
export type FieldKeyCode = Schema<"FieldKey">;
export type FieldLevelCode = Schema<"FieldLevel">;
export type PeopleKindKey = Schema<"PeopleKind">;
export type PeopleLevelCode = Schema<"PeopleLevel">;
/** A project as a name in a sentence, or *a project you cannot see*. */
export type ProjectLabel = Schema<"ProjectLabelBody">;
/** A role as a column of a table. */
export type RoleColumn = Schema<"RoleRefBody">;
/** Where the level of a cell was set. */
export type Origin = Schema<"LevelOriginBody">;

/** The level of one role for one field or kind. */
export interface Cell<Level extends string> {
  readonly role: string;
  readonly level: Level;
  readonly origin: Origin;
  /** In the table of an issue type: the level was set for that type, not for all types. */
  readonly for_type?: boolean;
}

/** A field or kind with the level of every role, in the order of the roles. */
export interface Row<Subject extends string, Level extends string> {
  readonly subject: Subject;
  /** The name of an own field; a field of the catalogue and a kind are named by messages. */
  readonly name?: string | null | undefined;
  /** The highest level the person who asks may set. */
  readonly yours: Level;
  readonly cells: readonly Cell<Level>[];
}

/**
 * What each role sees in one place: the field permissions and *Who sees people* have the same
 * shape, so one set of screens draws both.
 */
export interface LevelTable<Subject extends string, Level extends string> {
  readonly project?: ProjectLabel | null | undefined;
  /** The issue type the levels are for (an ID); none for all issue types. */
  readonly issue_type?: string | null | undefined;
  readonly roles: readonly RoleColumn[];
  readonly truncated: boolean;
  /** The levels a cell can be set to, the lowest first. */
  readonly levels: readonly Level[];
  readonly rows: readonly Row<Subject, Level>[];
}

/** One change of one cell; `null` takes away what a project set, so the level from above counts again. */
export interface Change<Subject extends string, Level extends string> {
  readonly role: string;
  readonly subject: Subject;
  readonly level: Level | null;
}

/** Whether a value the cache holds is a table of levels. */
export function isLevelTable<Subject extends string, Level extends string>(
  value: unknown,
): value is LevelTable<Subject, Level> {
  return (
    typeof value === "object" &&
    value !== null &&
    Array.isArray(Reflect.get(value, "rows")) &&
    Array.isArray(Reflect.get(value, "roles")) &&
    Array.isArray(Reflect.get(value, "levels"))
  );
}

/**
 * The table after a change, before the server has answered: the cell shows its new level, set for
 * the project (or the role's own, for the whole organization). Taking a setting away shows nothing
 * new at once: which level counts then is the server's to say.
 */
export function applied<Subject extends string, Level extends string>(
  before: LevelTable<Subject, Level> | undefined,
  change: Change<Subject, Level>,
): LevelTable<Subject, Level> | undefined {
  const { level } = change;
  if (before === undefined || level === null) {
    return before;
  }
  const origin: Origin =
    before.project === undefined || before.project === null
      ? { kind: "role-own" }
      : { kind: "set-here" };
  const forType = before.issue_type !== undefined && before.issue_type !== null;
  return {
    ...before,
    rows: before.rows.map((row) =>
      row.subject === change.subject
        ? {
            ...row,
            cells: row.cells.map((cell) =>
              cell.role === change.role
                ? { ...cell, level, origin, ...(forType ? { for_type: true } : {}) }
                : cell,
            ),
          }
        : row,
    ),
  };
}

/** Where a level stands in the choices of the table: the lowest is 0. */
export function rank<Level extends string>(levels: readonly Level[], level: Level): number {
  return levels.indexOf(level);
}

/** Whether the person who asks may set a cell to `level`: not above their own, whatever it is now. */
export function mayPick<Level extends string>(
  levels: readonly Level[],
  yours: Level,
  level: Level,
): boolean {
  return rank(levels, level) <= rank(levels, yours);
}
