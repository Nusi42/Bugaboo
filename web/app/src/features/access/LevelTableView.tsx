import { t } from "@bugaboo/messages";
import { Alert, alertTone, SaveStatus, SelectField, useLayout, partOf, part } from "@bugaboo/ui";
import { useState } from "react";

import { QueryGate } from "../shell/index.ts";
import { LevelPreview } from "./LevelPreview.tsx";
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
  /** The sentence of the preview under the table for a level, which names the fields or kinds the role has there. */
  readonly sentence: (level: Level, subjects: string) => string;
}

/**
 * The role the table opens on: Guest, the one that sees least, whose preview tells most, if the table has a role of
 * that name; else the first.
 */
function firstRole(table: LevelTable<string, string>): string {
  const guest = t("starting-role.guest");
  const named = table.roles.find((role) => role.name === guest);
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
    Omit<LevelTableViewProps<Subject, Level>, "levels">,
    Pick<LevelsOf<Subject, Level>, "edits"> {
  readonly table: LevelTable<Subject, Level>;
  readonly computer: boolean;
  readonly role: string;
  readonly onChoose: (role: string) => void;
}

/**
 * The table once it is there: notes about it, *Choose a role*, the matrix (the column of the chosen role marked) or the
 * list of the chosen role, and the preview of what that role sees. Both tables of levels choose a role this way.
 */
function TableBody<Subject extends string, Level extends string>({
  table,
  edits,
  computer,
  role,
  onChoose,
  sentence,
  ...view
}: TableBodyProps<Subject, Level>) {
  const hasRoles = table.roles.length > 0;
  return (
    <div {...partOf(part.stack)}>
      {table.truncated ? <Alert tone={alertTone.warning}>{t("access.roles-cut")}</Alert> : null}
      {table.roles.length === 0 ? <p {...partOf(part.muted)}>{t("access.no-roles")}</p> : null}
      {hasCeiling(table) ? <p {...partOf(part.muted)}>{t("access.ceiling")}</p> : null}
      {edits.problem === null ? null : <Alert tone={alertTone.danger}>{edits.problem}</Alert>}
      {hasRoles ? (
        // *Saving…* and *Saved* stand at the end of the line of the chooser's label, room it always has: nothing moves
        // when they come, and they take no line of their own (M5-F135).
        <div {...partOf(part.accessChooser)}>
          <SelectField
            label={t("common.choose-role")}
            choices={table.roles.map((known) => ({ value: known.id, label: known.name }))}
            value={role}
            onChange={onChoose}
          />
          <SaveStatus state={edits.saveState} labels={saveLabels()} />
        </div>
      ) : null}
      {computer ? (
        <LevelsMatrix table={table} onChange={edits.change} chosen={role} {...view} />
      ) : (
        <RoleLevels table={table} role={role} onChange={edits.change} {...view} />
      )}
      {hasRoles ? (
        <LevelPreview
          table={table}
          role={role}
          subjectName={view.subjectName}
          sentence={sentence}
        />
      ) : null}
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
          role={shown.roles.find((known) => known.id === chosen)?.id ?? firstRole(shown)}
          onChoose={setChosen}
        />
      )}
    </QueryGate>
  );
}
