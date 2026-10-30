import { t } from "@bugaboo/messages";
import { partOf, part } from "@bugaboo/ui";

import { useCallback, useId, useLayoutEffect, useRef, useState } from "react";

import { useOnResize } from "../../shared/use-on-resize.ts";

import { CellControl } from "./CellControl.tsx";
import type { Change, LevelTable, Row } from "./levels.ts";

/** Words of the HTML standard, not text for people. */
const column = "col";
const rowHeader = "row";
/** The value of `aria-current` for the column of the chosen role: one of a set, not a page. */
const chosenOfSet = "true";

export interface LevelsViewProps<Subject extends string, Level extends string> {
  readonly table: LevelTable<Subject, Level>;
  /** What a field or kind is called. */
  readonly subjectName: (subject: Subject, name: string | null | undefined) => string;
  /** What a level is called. */
  readonly levelName: (level: Level) => string;
  /** The heading of the first column: *Field* or *Kind of people information*. */
  readonly subjectHeading: string;
  /** What the table is, for the region that scrolls. */
  readonly tableName: string;
  readonly onChange: (change: Change<Subject, Level>) => void;
  /** The role chosen beside the matrix (for its preview): its column is scrolled into view and marked. */
  readonly chosen?: string | undefined;
}

interface MatrixRowProps<Subject extends string, Level extends string> extends Pick<
  LevelsViewProps<Subject, Level>,
  "table" | "subjectName" | "levelName" | "onChange" | "chosen"
> {
  readonly row: Row<Subject, Level>;
}

/** One field or kind: its name, then the level of every role. */
function MatrixRow<Subject extends string, Level extends string>({
  table,
  row,
  subjectName,
  levelName,
  onChange,
  chosen,
}: MatrixRowProps<Subject, Level>) {
  const inProject = table.project !== undefined && table.project !== null;
  const typed = table.issue_type !== undefined && table.issue_type !== null;
  const subject = subjectName(row.subject, row.name);
  return (
    <tr {...partOf(part.accessMatrixRow)}>
      <th scope={rowHeader} {...partOf(part.accessMatrixCell, { variants: ["row"] })}>
        {subject}
      </th>
      {row.cells.map((cell) => {
        const roleName = table.roles.find((known) => known.id === cell.role)?.name ?? "";
        return (
          <td
            key={cell.role}
            {...partOf(part.accessMatrixCell, { states: [cell.role === chosen && "chosen"] })}
          >
            <CellControl
              label={t("access.cell-label", { subject, role: roleName })}
              subjectName={subject}
              roleName={roleName}
              hideLabel
              levels={table.levels}
              levelName={levelName}
              cell={cell}
              yours={row.yours}
              inProject={inProject}
              typed={typed}
              onChange={(level) => {
                onChange({ role: cell.role, subject: row.subject, level });
              }}
            />
          </td>
        );
      })}
    </tr>
  );
}

/** Where a region that scrolls sideways is, and where a column inside it is, in pixels. */
interface Sideways {
  readonly scrollLeft: number;
  readonly clientWidth: number;
  /** The width of the names, which stay at the start. */
  readonly names: number;
  readonly left: number;
  readonly width: number;
}

/**
 * Where to scroll the region, as little as it takes, so that a column stands whole beside the names (UX §5.2); none
 * when it already does.
 */
function scrollFor(at: Sideways): number | undefined {
  const start = at.left - at.names;
  const end = at.left + at.width - at.clientWidth;
  if (at.scrollLeft > start) {
    return start;
  }
  return at.scrollLeft < end ? end : undefined;
}

/**
 * The region of the matrix: whether it has more to the side than it shows, kept up to date while the page is resized
 * and while the table grows or shrinks inside the region (a role is added), and the column of the chosen role
 * (its place in the table, 0 for none) scrolled into view whenever the choice changes.
 */
function useRegion(chosenAt: number) {
  const region = useRef<HTMLElement>(null);
  const table = useRef<HTMLTableElement>(null);
  const [scrolls, setScrolls] = useState(false);
  const measure = useCallback(() => {
    setScrolls((region.current?.scrollWidth ?? 0) > (region.current?.clientWidth ?? 0));
  }, []);
  const watched = useCallback(() => [region.current, table.current], []);
  useOnResize(measure, watched);
  useLayoutEffect(() => {
    const box = region.current;
    const cells = table.current?.rows[0]?.cells;
    const names = cells?.[0];
    const head = cells?.[chosenAt];
    if (box === null || names === undefined || head === undefined || chosenAt < 1) {
      return;
    }
    const left = scrollFor({
      scrollLeft: box.scrollLeft,
      clientWidth: box.clientWidth,
      names: names.offsetWidth,
      left: head.offsetLeft,
      width: head.offsetWidth,
    });
    // Only the region moves; the page stays where it is.
    if (left !== undefined) {
      box.scrollTo({ left });
    }
  }, [chosenAt]);
  return { region, table, scrolls };
}

/** The heads of the columns: what the rows are, then every role, the chosen one marked. */
function MatrixHead<Subject extends string, Level extends string>({
  table,
  subjectHeading,
  chosen,
}: Pick<LevelsViewProps<Subject, Level>, "table" | "subjectHeading" | "chosen">) {
  return (
    <thead>
      <tr {...partOf(part.accessMatrixRow)}>
        <th scope={column} {...partOf(part.accessMatrixCell, { variants: ["head"] })}>
          {subjectHeading}
        </th>
        {table.roles.map((role) => (
          <th
            key={role.id}
            scope={column}
            {...partOf(part.accessMatrixCell, {
              variants: ["head"],
              states: [role.id === chosen && "chosen"],
            })}
            {...(role.id === chosen ? { "aria-current": chosenOfSet } : {})}
          >
            {role.name}
          </th>
        ))}
      </tr>
    </thead>
  );
}

/**
 * The matrix of a computer: the fields (or kinds) down, the roles across, every cell a list of the
 * levels. The region scrolls on its own when there are more roles than fit; the lists inside it
 * can be reached with the keyboard, which scrolls to them.
 */
export function LevelsMatrix<Subject extends string, Level extends string>({
  table,
  subjectHeading,
  tableName,
  ...rest
}: LevelsViewProps<Subject, Level>) {
  // The first column holds the names, so the column of a role is one after its place in the list.
  const {
    region,
    table: tableElement,
    scrolls,
  } = useRegion(table.roles.findIndex((role) => role.id === rest.chosen) + 1);
  const noteId = useId();
  return (
    <>
      {scrolls ? (
        <p id={noteId} {...partOf(part.muted)}>
          {t("access.scrolls")}
        </p>
      ) : null}
      <section
        ref={region}
        {...partOf(part.accessScroll)}
        aria-label={tableName}
        {...(scrolls ? { "aria-describedby": noteId } : {})}
      >
        <table ref={tableElement} {...partOf(part.accessMatrix)}>
          <MatrixHead table={table} subjectHeading={subjectHeading} chosen={rest.chosen} />
          <tbody>
            {table.rows.map((row) => (
              <MatrixRow key={row.subject} table={table} row={row} {...rest} />
            ))}
          </tbody>
        </table>
      </section>
    </>
  );
}
