import { t } from "@bugaboo/messages";
import { Card, SelectField, partOf, part } from "@bugaboo/ui";
import { useRef } from "react";

import { useDrop } from "react-aria-components";

import { dropOptions } from "./dnd.ts";
import { DraggableHead } from "./DraggableHead.tsx";
import type { EditorActions, EditorContext } from "./editor-actions.ts";
import type { Row } from "./editor-model.ts";
import { fieldLabel } from "./field-label.ts";

/** What a list that adds a field shows when nothing is picked, and what it holds then. */
const noSection = "";

interface AvailableProps {
  readonly rows: readonly Row[];
  readonly context: EditorContext;
  readonly actions: EditorActions;
}

/** A field that is on no section: the handle to drag it in, and the list that adds it to a section. */
function AvailableRow({
  row,
  context,
  actions,
}: Pick<AvailableProps, "context" | "actions"> & { readonly row: Row }) {
  const name = fieldLabel(row.field, row.name);
  return (
    <li {...partOf(part.layoutField)}>
      <DraggableHead field={row.field} name={name} canDrag={context.canDrag}>
        <span {...partOf(part.layoutFieldName)}>{name}</span>
      </DraggableHead>
      <SelectField
        label={t("layouts.field-add-to", { name })}
        choices={[
          { value: noSection, label: t("layouts.field-add-to", { name }) },
          ...context.moves,
        ]}
        value={noSection}
        onChange={(section) => {
          if (section !== noSection) {
            actions.place(row.field, { section, index: Number.MAX_SAFE_INTEGER });
          }
        }}
      />
    </li>
  );
}

/**
 * The fields that are on no section of the layout. Dropping a field here takes it off the form;
 * a field is added to a section with its list or by dragging it onto the section.
 */
export function AvailableFields({ rows, context, actions }: AvailableProps) {
  const ref = useRef<HTMLDivElement>(null);
  const drop = useDrop({ ref, ...dropOptions(context.canDrag, actions.unplace) });
  return (
    <Card heading={t("layouts.available")}>
      <div
        ref={ref}
        {...drop.dropProps}
        {...partOf(part.layoutSlot, { states: [drop.isDropTarget && "over"] })}
        aria-label={t("layouts.available")}
      />
      {rows.length === 0 ? <p {...partOf(part.muted)}>{t("layouts.available-none")}</p> : null}
      <ul {...partOf(part.layoutSectionFields)} aria-label={t("layouts.available")}>
        {rows.map((row) => (
          <AvailableRow key={row.field} row={row} context={context} actions={actions} />
        ))}
      </ul>
    </Card>
  );
}
