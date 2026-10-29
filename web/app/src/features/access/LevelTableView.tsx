import { t } from "@bugaboo/messages";
import { Alert, alertTone, SaveStatus, SelectField, useLayout, partOf, part } from "@bugaboo/ui";
import { useState, type ReactNode } from "react";

import { QueryGate } from "../shell/index.ts";
import { LevelsMatrix, type LevelsViewProps } from "./LevelsMatrix.tsx";
import { rank, type LevelTable } from "./levels.ts";
import { RoleLevels } from "./RoleLevels.tsx";
import type { LevelsOf } from "./use-levels.ts";
import { saveLabels } from "../../shared/save-labels.ts";

export interface LevelTableViewProps<Subject extends string, Level extends string> extends Omit<
  LevelsViewProps<Subject, Level>,
  "table" | "onChange"
> {
  readonly levels: LevelsOf<Subject, Level>;
  /** The name of the role the table opens on a phone, if it has a role of that name; else the first role. */
  readonly preferredRole?: string;
  /**
   * The table has something to show for the chosen role (a preview), so a computer has the chooser of a role too;
   * else the chooser is only for phones and tablets.
   */
  readonly previewsRole?: boolean;
  /** What follows the table, for the role that is chosen: a preview. */
  readonly after?: (table: LevelTable<Subject, Level>, role: string) => ReactNode;
}

/** The role the table opens on: the preferred one by name, else the first. */
function firstRole(table: LevelTable<string, string>, preferred: string | undefined): string {
  const named = table.roles.find((role) => role.name === preferred);
  return named?.id ?? table.roles[0]?.id ?? "";
}

/** Whether the person who asks may set less than the highest level somewhere: then the table says so. */
function hasCeiling<Subject extends string, Level extends string>(
  table: LevelTable<Subject, Level>,
): boolean {
  return table.rows.some((row) => rank(table.levels, row.yours) < table.levels.length - 1);
}

interface TableBodyProps<Subject extends string, Level extends string>
  extends
    Omit<LevelTableViewProps<Subject, Level>, "levels" | "preferredRole">,
    Pick<LevelsOf<Subject, Level>, "edits"> {
  readonly table: LevelTable<Subject, Level>;
  readonly computer: boolean;
  readonly role: string;
  readonly onChoose: (role: string) => void;
}

/** The table once it is there: notes about it, the chooser of a role, the matrix or the list. */
function TableBody<Subject extends string, Level extends string>({
  table,
  edits,
  computer,
  role,
  onChoose,
  previewsRole = false,
  after,
  ...view
}: TableBodyProps<Subject, Level>) {
  const showChooser = table.roles.length > 0 && (!computer || previewsRole);
  return (
    <div {...partOf(part.stack)}>
      {table.truncated ? <Alert tone={alertTone.warning}>{t("access.roles-cut")}</Alert> : null}
      {table.roles.length === 0 ? <p {...partOf(part.muted)}>{t("access.no-roles")}</p> : null}
      {hasCeiling(table) ? <p {...partOf(part.muted)}>{t("access.ceiling")}</p> : null}
      <SaveStatus state={edits.saveState} labels={saveLabels()} />
      {edits.problem === null ? null : <Alert tone={alertTone.danger}>{edits.problem}</Alert>}
      {showChooser ? (
        <SelectField
          label={t("common.choose-role")}
          choices={table.roles.map((known) => ({ value: known.id, label: known.name }))}
          value={role}
          onChange={onChoose}
        />
      ) : null}
      {computer ? (
        <LevelsMatrix table={table} onChange={edits.change} {...view} />
      ) : (
        <RoleLevels table={table} role={role} onChange={edits.change} {...view} />
      )}
      {table.roles.length > 0 && after !== undefined ? after(table, role) : null}
    </div>
  );
}

/**
 * The screen of a table of levels (SEC-04, PRIV-12): the matrix on a computer, one role at a time
 * as a list on a phone and a tablet. Changes show at once; what the server refuses comes back with
 * its plain message.
 */
export function LevelTableView<Subject extends string, Level extends string>({
  levels,
  preferredRole,
  ...view
}: LevelTableViewProps<Subject, Level>) {
  const computer = useLayout() === "computer";
  const [chosen, setChosen] = useState<string>();
  const { table, failed, edits } = levels;
  return (
    <QueryGate error={failed} data={table}>
      {(shown) => (
        <TableBody
          {...view}
          table={shown}
          edits={edits}
          computer={computer}
          role={
            shown.roles.find((known) => known.id === chosen)?.id ?? firstRole(shown, preferredRole)
          }
          onChoose={setChosen}
        />
      )}
    </QueryGate>
  );
}
