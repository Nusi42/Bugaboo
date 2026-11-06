import { t } from "@bugaboo/messages";
import { Button, buttonLook, partOf, part } from "@bugaboo/ui";

import type { OwnField } from "./fields.ts";
import { ValueEditor, type Sent } from "./ValueEditor.tsx";
import { ValueView } from "./ValueView.tsx";

/** Where the value that stands in for a missing one comes from, and what it is. */
function InheritedNote({
  field,
  value,
  source,
}: {
  readonly field: OwnField;
  readonly value: unknown;
  readonly source: string | undefined;
}) {
  return (
    <div {...partOf(part.row, { variants: ["note"] })}>
      <p {...partOf(part.settingsSource)}>
        {source === undefined
          ? t("fields.time-default-hidden-source")
          : t("fields.time-default-source", { project: source })}
      </p>
      <ValueView field={field} value={value} />
    </div>
  );
}

export interface TimeDefaultLineProps {
  readonly field: OwnField;
  readonly project: string;
  /** The value this project sets itself. */
  readonly own: unknown;
  /** The value that comes from a project above, which the server worked out; nothing when none does. */
  readonly inherited: unknown;
  /** The name of the project that sets the inherited value; nothing when the person cannot see it. */
  readonly source: string | undefined;
  readonly onCommit: (field: OwnField, value: Sent) => void;
}

/**
 * One field of time entries as a default of a project: the name above its box like every setting,
 * what this project sets or, while it sets nothing, what the project above does, and *Reset to
 * default* at the edge of the card where this project has a value of its own.
 */
export function TimeDefaultLine({
  field,
  project,
  own,
  inherited,
  source,
  onCommit,
}: TimeDefaultLineProps) {
  const setHere = own !== undefined && own !== null;
  const fromAbove = inherited ?? undefined;
  return (
    <li {...partOf(part.box)}>
      <div {...partOf(part.stack)}>
        <ValueEditor
          field={field}
          project={project}
          value={own}
          {...(!setHere && fromAbove !== undefined
            ? { emptyLabel: t("fields.time-default-from-above") }
            : {})}
          onCommit={(sent) => {
            onCommit(field, sent);
          }}
        />
        {setHere || fromAbove === undefined ? null : (
          <InheritedNote field={field} value={fromAbove} source={source} />
        )}
        {setHere ? (
          <div {...partOf(part.row)}>
            <Button
              look={buttonLook.plain}
              aria-label={t("setting.ui.reset-name", { name: field.name })}
              onPress={() => {
                onCommit(field, null);
              }}
            >
              {t("setting.ui.reset")}
            </Button>
          </div>
        ) : null}
      </div>
    </li>
  );
}
