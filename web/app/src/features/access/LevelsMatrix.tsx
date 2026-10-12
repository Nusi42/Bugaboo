import { t } from "@bugaboo/messages";
import { partOf, part } from "@bugaboo/ui";

import { CellControl } from "./CellControl.tsx";
import type { Change, LevelTable, Row } from "./levels.ts";

/** Words of the HTML standard, not text for people. */
const column = "col";
const rowHeader = "row";

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
}

interface MatrixRowProps<Subject extends string, Level extends string> extends Pick<
  LevelsViewProps<Subject, Level>,
  "table" | "subjectName" | "levelName" | "onChange"
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
          <td key={cell.role} {...partOf(part.accessMatrixCell)}>
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
  return (
    <section {...partOf(part.accessScroll)} aria-label={tableName}>
      <table {...partOf(part.accessMatrix)}>
        <thead>
          <tr {...partOf(part.accessMatrixRow)}>
            <th scope={column} {...partOf(part.accessMatrixCell, { variants: ["head"] })}>
              {subjectHeading}
            </th>
            {table.roles.map((role) => (
              <th
                key={role.id}
                scope={column}
                {...partOf(part.accessMatrixCell, { variants: ["head"] })}
              >
                {role.name}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {table.rows.map((row) => (
            <MatrixRow key={row.subject} table={table} row={row} {...rest} />
          ))}
        </tbody>
      </table>
    </section>
  );
}
