import { t } from "@bugaboo/messages";
import {
  buttonLook,
  ConfirmDialog,
  FieldValue,
  IconButton,
  iconName,
  SelectField,
  partOf,
  part,
} from "@bugaboo/ui";
import { useState, type ReactNode } from "react";

import { DraggableHead } from "../../shared/drag-and-drop.tsx";
import { badgeLook, choiceKind, plainText, type Choice, type OwnField } from "./fields.ts";
import { ChoiceLook } from "./ChoiceLook.tsx";
import { ChoiceName } from "./ChoiceName.tsx";
import type { Place } from "./choice-order.ts";
import { useChangeChoice } from "./use-change-field.ts";

/** Sends a change of the choice, on the version of the field the person looked at. */
type Send = (more: Readonly<Record<string, unknown>>) => void;

export interface ChoiceRowProps {
  readonly field: OwnField;
  readonly choice: Choice;
  /** Where it stands among the choices put in order by hand; none for a choice with a number. */
  readonly place: Place | undefined;
  readonly onMove: (by: -1 | 1) => void;
  /** Whether the choice can be dragged to another place: not on a phone, where the buttons are the way. */
  readonly canDrag: boolean;
  /** What comes first in the item: the place before it where a dragged choice can be dropped. */
  readonly before: ReactNode;
  /** The other choices a value can be combined into. */
  readonly others: readonly Choice[];
  readonly onCombine: (into: string) => void;
}

/** The choice as its field shows it, with the buttons that move it. */
function ChoiceTitle({
  field,
  choice,
  place,
  canDrag,
  onMove,
}: Pick<ChoiceRowProps, "field" | "choice" | "place" | "canDrag" | "onMove">) {
  return (
    <DraggableHead
      kind={choiceKind}
      id={choice.id}
      label={t("fields.choice-drag", { name: choice.shown_as })}
      canDrag={canDrag && place !== undefined}
    >
      <FieldValue
        look={field.look === plainText ? badgeLook : field.look}
        text={choice.name}
        number={choice.number}
        color={choice.color}
        icon={choice.icon ?? undefined}
      />
      {place === undefined ? null : (
        <>
          <IconButton
            icon={iconName.up}
            label={t("fields.choice-up", { name: choice.shown_as })}
            isDisabled={place.first}
            onPress={() => {
              onMove(-1);
            }}
          />
          <IconButton
            icon={iconName.down}
            label={t("fields.choice-down", { name: choice.shown_as })}
            isDisabled={place.last}
            onPress={() => {
              onMove(1);
            }}
          />
        </>
      )}
    </DraggableHead>
  );
}

/** *Combine with another choice*: every value of this choice moves to the one picked, after a question. */
function ChoiceCombine({
  choice,
  others,
  onCombine,
}: Pick<ChoiceRowProps, "choice" | "others" | "onCombine">) {
  const [into, setInto] = useState(others[0]?.id ?? "");
  return (
    <div {...partOf(part.row)}>
      <SelectField
        label={t("fields.combine-into")}
        choices={others.map((other) => ({ value: other.id, label: other.shown_as }))}
        value={into}
        onChange={setInto}
      />
      <ConfirmDialog
        triggerLabel={t("fields.combine")}
        triggerName={t("fields.combine-name", { name: choice.shown_as })}
        title={t("fields.combine-title", { name: choice.shown_as })}
        text={t("fields.combine-text", {
          name: choice.shown_as,
          into: others.find((other) => other.id === into)?.shown_as ?? "",
        })}
        confirmLabel={t("fields.combine")}
        cancelLabel={t("common.cancel")}
        confirmLook={buttonLook.danger}
        onConfirm={() => {
          onCombine(into);
        }}
      />
    </div>
  );
}

/** One choice of the field: its name, color, icon, place in the order, *Combine* and *Retire*. */
export function ChoiceRow(props: ChoiceRowProps) {
  const { field, choice, others } = props;
  const change = useChangeChoice();
  const send: Send = (more) => {
    change.mutate({
      params: { path: { field: field.id, choice: choice.id } },
      body: { version: field.version, ...more },
    });
  };
  return (
    <li {...partOf(part.fieldChoice)}>
      {props.before}
      <ChoiceTitle {...props} />
      <ChoiceName choice={choice} send={send} pending={change.isPending} failure={change.error} />
      <ChoiceLook choice={choice} send={send} />
      {others.length > 0 && !choice.retired ? (
        <ChoiceCombine choice={choice} others={others} onCombine={props.onCombine} />
      ) : null}
    </li>
  );
}
