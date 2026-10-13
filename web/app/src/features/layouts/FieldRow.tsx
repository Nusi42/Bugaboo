import { t } from "@bugaboo/messages";
import { Checkbox, IconButton, iconName, SelectField, partOf, part } from "@bugaboo/ui";

import type { Schema } from "@bugaboo/api-client";
import { DraggableHead } from "./DraggableHead.tsx";
import { fieldLabel } from "./field-label.ts";
import type { FormKind } from "./layouts.ts";
import type { Row } from "./editor-model.ts";

/** A role the editor can tick. */
type Role = Schema<"EditorRoleBody">;

/** Where a field can be moved to with the list: a section, by its number and title. */
export interface MoveChoice {
  readonly value: string;
  readonly label: string;
}

export interface FieldRowProps {
  readonly row: Row;
  readonly kind: FormKind;
  readonly roles: readonly Role[];
  /** The sections the field can be moved to. */
  readonly sections: readonly MoveChoice[];
  readonly first: boolean;
  readonly last: boolean;
  /** Whether the row can be dragged: not on a phone, where the buttons are the way. */
  readonly canDrag: boolean;
  readonly onNudge: (by: -1 | 1) => void;
  readonly onMoveTo: (section: string) => void;
  readonly onRemove: () => void;
  readonly onRole: (role: string, on: boolean) => void;
}

/** The roles of a field with a box each, and the words that say where field permissions win. */
function RoleBoxes({
  row,
  kind,
  roles,
  onRole,
}: Pick<FieldRowProps, "row" | "kind" | "roles" | "onRole">) {
  const name = fieldLabel(row.field, row.name);
  const limited = roles.filter((role) => row.limitedFor.includes(role.id));
  return (
    <>
      <fieldset {...partOf(part.layoutFieldRoles)}>
        <legend>
          {t(kind === "issue-page" ? "layouts.roles-page" : "layouts.roles-form", { name })}
        </legend>
        {roles.map((role) => (
          <Checkbox
            key={role.id}
            label={role.name}
            isSelected={row.roles.includes(role.id)}
            onChange={(on) => {
              onRole(role.id, on);
            }}
          />
        ))}
      </fieldset>
      {limited.map((role) => (
        <p key={role.id} {...partOf(part.layoutFieldLimited)}>
          {t(kind === "issue-page" ? "layouts.limited-page" : "layouts.limited-form", {
            role: role.name,
            field: name,
          })}
        </p>
      ))}
    </>
  );
}

/** What a move-to list shows when nothing is picked, and what it holds then. */
const noSection = "";

/** The list that moves a field to the end of another section: the way to do a drag with the list. */
function MoveTo({
  name,
  sections,
  onMoveTo,
}: Pick<FieldRowProps, "sections" | "onMoveTo"> & { readonly name: string }) {
  return (
    <div {...partOf(part.layoutFieldActions)}>
      <SelectField
        label={t("layouts.field-move-to", { name })}
        choices={[{ value: noSection, label: t("layouts.field-move-to", { name }) }, ...sections]}
        value={noSection}
        onChange={(section) => {
          if (section !== noSection) {
            onMoveTo(section);
          }
        }}
      />
    </div>
  );
}

/** The handle, the name and the buttons of a field: up, down and off the form. */
function FieldHead({ props, name }: { readonly props: FieldRowProps; readonly name: string }) {
  const { row, first, last, canDrag, onNudge, onRemove } = props;
  return (
    <DraggableHead field={row.field} name={name} canDrag={canDrag}>
      <span {...partOf(part.layoutFieldName)}>{name}</span>
      <IconButton
        icon={iconName.up}
        label={t("layouts.field-up", { name })}
        isDisabled={first}
        onPress={() => {
          onNudge(-1);
        }}
      />
      <IconButton
        icon={iconName.down}
        label={t("layouts.field-down", { name })}
        isDisabled={last}
        onPress={() => {
          onNudge(1);
        }}
      />
      <IconButton
        icon={iconName.close}
        label={t("layouts.field-remove", { name })}
        onPress={onRemove}
      />
    </DraggableHead>
  );
}

/** One field of a section: its name, the handle to drag it, the buttons for everything a drag does, and its roles. */
export function FieldRow(props: FieldRowProps) {
  const { row, kind, sections, onMoveTo, roles, onRole } = props;
  const name = fieldLabel(row.field, row.name);
  return (
    <li {...partOf(part.layoutField)}>
      <FieldHead props={props} name={name} />
      {sections.length > 1 ? <MoveTo name={name} sections={sections} onMoveTo={onMoveTo} /> : null}
      {row.required ? <p {...partOf(part.muted)}>{t("layouts.field-required")}</p> : null}
      <RoleBoxes row={row} kind={kind} roles={roles} onRole={onRole} />
    </li>
  );
}
