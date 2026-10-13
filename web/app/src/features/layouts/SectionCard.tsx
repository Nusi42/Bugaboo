import { t } from "@bugaboo/messages";
import { Card, IconButton, iconName, TextField, partOf, part } from "@bugaboo/ui";
import { DropSlot } from "../../shared/drag-and-drop.tsx";
import { fieldKind } from "./field-kind.ts";
import type { EditorActions, EditorContext } from "./editor-actions.ts";
import type { Row, Section } from "./editor-model.ts";
import { FieldRow } from "./FieldRow.tsx";

export interface SectionCardProps {
  readonly section: Section;
  /** Its place among the sections, from 1. */
  readonly number: number;
  readonly count: number;
  readonly context: EditorContext;
  readonly actions: EditorActions;
}

/** The title and tab of a section, and the buttons that move or remove it. */
function SectionHead({ section, number, count, actions }: SectionCardProps) {
  const label = String(number);
  return (
    <div {...partOf(part.layoutSectionHead)}>
      <TextField
        label={t("layouts.section-title")}
        value={section.title}
        onChange={(title) => {
          actions.editSection(section.key, { title });
        }}
      />
      <TextField
        label={t("layouts.section-tab")}
        description={t("layouts.section-tab-help")}
        value={section.tab}
        onChange={(tab) => {
          actions.editSection(section.key, { tab });
        }}
      />
      <IconButton
        icon={iconName.up}
        label={t("layouts.section-up", { number: label })}
        isDisabled={number === 1}
        onPress={() => {
          actions.nudgeSection(section.key, -1);
        }}
      />
      <IconButton
        icon={iconName.down}
        label={t("layouts.section-down", { number: label })}
        isDisabled={number === count}
        onPress={() => {
          actions.nudgeSection(section.key, 1);
        }}
      />
      <IconButton
        icon={iconName.close}
        label={t("layouts.section-remove", { number: label })}
        onPress={() => {
          actions.removeSection(section.key);
        }}
      />
    </div>
  );
}

/** A field of a section, with the place before it where a dragged field can be dropped. */
function RowAt({
  row,
  at,
  section,
  context,
  actions,
}: Pick<SectionCardProps, "section" | "context" | "actions"> & {
  readonly row: Row;
  readonly at: number;
}) {
  const moves = context.moves.filter((move) => move.value !== section.key);
  return (
    <FieldRow
      row={row}
      kind={context.kind}
      roles={context.roles}
      sections={moves}
      first={at === 0}
      last={at === section.rows.length - 1}
      canDrag={context.canDrag}
      before={
        <DropSlot
          kind={fieldKind}
          enabled={context.canDrag}
          onDrop={(field) => {
            actions.place(field, { section: section.key, index: at });
          }}
        />
      }
      onNudge={(by) => {
        actions.nudgeField(row.field, by);
      }}
      onMoveTo={(target) => {
        actions.place(row.field, { section: target, index: Number.MAX_SAFE_INTEGER });
      }}
      onRemove={() => {
        actions.unplace(row.field);
      }}
      onRole={(role, on) => {
        actions.setRole(row.field, role, on);
      }}
    />
  );
}

/** The fields of a section in order, with the places a field can be dropped between them. */
function Rows({ section, number, context, actions }: SectionCardProps) {
  return (
    <>
      {section.rows.length === 0 ? (
        <p {...partOf(part.muted)}>{t("layouts.section-empty")}</p>
      ) : null}
      <ul
        {...partOf(part.layoutSectionFields)}
        aria-label={t("layouts.fields-of", { number: String(number) })}
      >
        {section.rows.map((row, at) => (
          <RowAt
            key={row.field}
            row={row}
            at={at}
            section={section}
            context={context}
            actions={actions}
          />
        ))}
      </ul>
      <DropSlot
        kind={fieldKind}
        enabled={context.canDrag}
        onDrop={(field) => {
          actions.place(field, { section: section.key, index: section.rows.length });
        }}
      />
    </>
  );
}

/** One section of the layout: its words, its fields in order, and the places a field can be dropped. */
export function SectionCard(props: SectionCardProps) {
  return (
    <Card heading={t("layouts.section", { number: String(props.number) })}>
      <SectionHead {...props} />
      <Rows {...props} />
    </Card>
  );
}
