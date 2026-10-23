import { t } from "@bugaboo/messages";
import { Button, buttonLook, SelectField, partOf, part } from "@bugaboo/ui";

import { mayPick, type Cell } from "./levels.ts";
import { originText } from "./origin-text.ts";

export interface CellControlProps<Level extends string> {
  /** The name of the list of levels for screen readers and voice control: "Cost center for Contributor". */
  readonly label: string;
  /** The name of the field or kind alone, for the button that takes a project's setting away. */
  readonly subjectName: string;
  readonly roleName: string;
  /** The name shows only for screen readers: the table already says what the cell is about. */
  readonly hideLabel: boolean;
  readonly levels: readonly Level[];
  readonly levelName: (level: Level) => string;
  readonly cell: Cell<Level>;
  /** The highest level the person may set. */
  readonly yours: Level;
  /** The table is the one of a project, which can set its own levels and take them away. */
  readonly inProject: boolean;
  /** The table is the one of an issue type: only a level set for that type can be taken away. */
  readonly typed: boolean;
  /** A level to set, or `null` to take away what the project set. */
  readonly onChange: (level: Level | null) => void;
}

/** The level of one role for one field or kind: a list of the levels, where it was set, and *Reset*. */
export function CellControl<Level extends string>({
  label,
  subjectName,
  roleName,
  hideLabel,
  levels,
  levelName,
  cell,
  yours,
  inProject,
  typed,
  onChange,
}: CellControlProps<Level>) {
  const origin = originText(cell.origin);
  const resettable = typed
    ? cell.for_type === true &&
      (inProject ? cell.origin.kind === "set-here" : cell.origin.kind === "role-own")
    : inProject && cell.origin.kind === "set-here";
  return (
    <div {...partOf(part.accessCell)}>
      <SelectField
        label={label}
        hideLabel={hideLabel}
        {...(origin === undefined ? {} : { description: origin })}
        choices={levels.map((level) => ({
          value: level,
          label: levelName(level),
          disabled: level !== cell.level && !mayPick(levels, yours, level),
        }))}
        value={cell.level}
        onChange={onChange}
      />
      {resettable ? (
        <Button
          look={buttonLook.quiet}
          aria-label={t("access.reset-label", { subject: subjectName, role: roleName })}
          onPress={() => {
            onChange(null);
          }}
        >
          {t("access.reset")}
        </Button>
      ) : null}
    </div>
  );
}
