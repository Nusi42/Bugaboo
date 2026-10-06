import { t } from "@bugaboo/messages";
import { Alert, SaveStatus, SelectField, useLayout } from "@bugaboo/ui";
import { useState, type ReactNode } from "react";

import { messageOf } from "../../shared/errors.ts";
import styles from "./Access.module.css";
import { LevelsMatrix, type LevelsViewProps } from "./LevelsMatrix.tsx";
import { rank, type LevelTable } from "./levels.ts";
import { RoleLevels } from "./RoleLevels.tsx";
import type { LevelsOf } from "./use-levels.ts";

export interface LevelTableViewProps<Subject extends string, Level extends string> extends Omit<
  LevelsViewProps<Subject, Level>,
  "table" | "onChange"
> {
  readonly levels: LevelsOf<Subject, Level>;
  /** The name of the role the table opens on a phone, if it has a role of that name; else the first role. */
  readonly preferredRole?: string;
  /**
   * What a computer says about the chooser of a role when the table has something to show for the
   * chosen role (a preview), else the chooser is only for phones and tablets.
   */
  readonly previewLabel?: string;
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

/**
 * The screen of a table of levels (SEC-04, PRIV-12): the matrix on a computer, one role at a time
 * as a list on a phone and a tablet. Changes show at once; what the server refuses comes back with
 * its plain message.
 */
export function LevelTableView<Subject extends string, Level extends string>({
  levels,
  preferredRole,
  previewLabel,
  after,
  ...view
}: LevelTableViewProps<Subject, Level>) {
  const computer = useLayout() === "computer";
  const [chosen, setChosen] = useState<string>();
  const { table, failed, edits } = levels;
  if (failed !== null && failed !== undefined) {
    return <Alert danger>{messageOf(failed)}</Alert>;
  }
  if (table === undefined) {
    return <p className={styles.muted}>{t("common.loading")}</p>;
  }
  const role =
    table.roles.find((known) => known.id === chosen)?.id ?? firstRole(table, preferredRole);
  const showChooser = table.roles.length > 0 && (!computer || previewLabel !== undefined);
  return (
    <div className={styles.stack}>
      {table.truncated ? <Alert warning>{t("access.roles-cut")}</Alert> : null}
      {table.roles.length === 0 ? <p className={styles.muted}>{t("access.no-roles")}</p> : null}
      {hasCeiling(table) ? <p className={styles.muted}>{t("access.ceiling")}</p> : null}
      <SaveStatus
        state={edits.saveState}
        labels={{
          saving: t("save.saving"),
          saved: t("save.saved"),
          "not-saved": t("save.not-saved"),
        }}
      />
      {edits.problem === null ? null : <Alert danger>{edits.problem}</Alert>}
      {showChooser ? (
        <SelectField
          label={computer && previewLabel !== undefined ? previewLabel : t("access.role-chooser")}
          choices={table.roles.map((known) => ({ value: known.id, label: known.name }))}
          value={role}
          onChange={setChosen}
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
