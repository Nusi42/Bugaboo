import { partOf, part } from "@bugaboo/ui";

import { CellControl } from "./CellControl.tsx";
import type { LevelsViewProps } from "./LevelsMatrix.tsx";

export interface RoleLevelsProps<Subject extends string, Level extends string> extends Omit<
  LevelsViewProps<Subject, Level>,
  "subjectHeading"
> {
  /** The ID of the role whose list is shown. */
  readonly role: string;
}

/**
 * The matrix on a phone and on a tablet: the list of one role (the role is chosen above it), each
 * field or kind with its level, so nothing has to scroll sideways.
 */
export function RoleLevels<Subject extends string, Level extends string>({
  table,
  role,
  tableName,
  subjectName,
  levelName,
  onChange,
}: RoleLevelsProps<Subject, Level>) {
  const inProject = table.project !== undefined && table.project !== null;
  const roleName = table.roles.find((known) => known.id === role)?.name ?? "";
  return (
    <ul {...partOf(part.list)} aria-label={tableName}>
      {table.rows.map((row) => {
        const cell = row.cells.find((known) => known.role === role);
        return cell === undefined ? null : (
          <li key={row.subject} {...partOf(part.accessListItem)}>
            <CellControl
              label={subjectName(row.subject)}
              subjectName={subjectName(row.subject)}
              roleName={roleName}
              hideLabel={false}
              levels={table.levels}
              levelName={levelName}
              cell={cell}
              yours={row.yours}
              inProject={inProject}
              onChange={(level) => {
                onChange({ role, subject: row.subject, level });
              }}
            />
          </li>
        );
      })}
    </ul>
  );
}
