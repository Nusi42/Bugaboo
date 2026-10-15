import { t } from "@bugaboo/messages";
import {
  Alert,
  alertTone,
  Card,
  Form,
  SubmitButton,
  TextField,
  useLayout,
  partOf,
  part,
} from "@bugaboo/ui";
import { useState } from "react";

import { DropSlot } from "../../shared/drag-and-drop.tsx";
import { messageOf } from "../../shared/errors.ts";
import { ChoiceRow } from "./ChoiceRow.tsx";
import { choiceKind, type OwnField } from "./fields.ts";
import { useAddChoice, useCombineChoices, useReorderChoices } from "./use-change-field.ts";

/** The new order after the choice at `from` goes to `to`. */
function moved(ids: readonly string[], from: number, to: number): string[] {
  const next = [...ids];
  const [taken] = next.splice(from, 1);
  if (taken !== undefined) {
    next.splice(to, 0, taken);
  }
  return next;
}

/** Puts the choices in the order the buttons ask for. */
function useChoiceOrder(field: OwnField) {
  const reorder = useReorderChoices();
  const ids = field.choices.map((choice) => choice.id);
  return {
    error: reorder.error,
    /** Choices can be dragged: not on a phone, where the buttons are the way. */
    canDrag: useLayout() !== "phone",
    move: (at: number, by: number) => {
      reorder.mutate({
        params: { path: { field: field.id } },
        body: { version: field.version, order: moved(ids, at, at + by) },
      });
    },
    /** A choice was dropped before the choice at `before` (the end of the list when it is `ids.length`). */
    drop: (dropped: string, before: number) => {
      const from = ids.indexOf(dropped);
      if (from < 0) {
        return;
      }
      reorder.mutate({
        params: { path: { field: field.id } },
        body: {
          version: field.version,
          order: moved(ids, from, from < before ? before - 1 : before),
        },
      });
    },
  };
}

/** The form that adds a choice at the end. */
function AddChoice({ field }: { readonly field: OwnField }) {
  const [name, setName] = useState("");
  const add = useAddChoice();
  return (
    <Form
      onSubmit={() => {
        add.mutate(
          {
            params: { path: { field: field.id } },
            body: { version: field.version, name: name.trim() },
          },
          {
            onSuccess: () => {
              setName("");
            },
          },
        );
      }}
    >
      <TextField
        label={t("fields.add-choice")}
        requiredMessage={t("error.field.required")}
        value={name}
        onChange={setName}
      />
      <div {...partOf(part.row)}>
        <SubmitButton isPending={add.isPending}>{t("fields.add-choice-button")}</SubmitButton>
      </div>
      {add.isError ? <Alert tone={alertTone.danger}>{messageOf(add.error)}</Alert> : null}
    </Form>
  );
}

/**
 * *Choices* of the field page (CF-04, CF-07): reorder by dragging or with the buttons, a color and
 * an icon for each, *Combine with another choice* (moves every value, and says so in the history of
 * every issue it touches) and *Retire* (not offered for new values, kept on the old ones).
 */
export function ChoicesSection({ field }: { readonly field: OwnField }) {
  const order = useChoiceOrder(field);
  const combine = useCombineChoices();
  const failure = order.error ?? combine.error;
  return (
    <Card heading={t("fields.choices")}>
      <p {...partOf(part.muted)}>{t("fields.choices-help")}</p>
      {failure === null ? null : <Alert tone={alertTone.danger}>{messageOf(failure)}</Alert>}
      <ul {...partOf(part.list)} aria-label={t("fields.choices")}>
        {field.choices.map((choice, at) => (
          <ChoiceRow
            key={choice.id}
            field={field}
            choice={choice}
            first={at === 0}
            last={at === field.choices.length - 1}
            canDrag={order.canDrag}
            before={
              <DropSlot
                kind={choiceKind}
                enabled={order.canDrag}
                onDrop={(dropped) => {
                  order.drop(dropped, at);
                }}
              />
            }
            others={field.choices.filter((other) => other.id !== choice.id && !other.retired)}
            onMove={(by) => {
              order.move(at, by);
            }}
            onCombine={(into) => {
              combine.mutate({
                params: { path: { field: field.id, choice: choice.id } },
                body: { version: field.version, into },
              });
            }}
          />
        ))}
      </ul>
      <DropSlot
        kind={choiceKind}
        enabled={order.canDrag}
        onDrop={(dropped) => {
          order.drop(dropped, field.choices.length);
        }}
      />
      <AddChoice field={field} />
    </Card>
  );
}
