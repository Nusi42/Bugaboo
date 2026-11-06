import { t } from "@bugaboo/messages";
import { Alert, alertTone, Card, useLayout, partOf, part } from "@bugaboo/ui";

import { DropSlot } from "../../shared/drag-and-drop.tsx";
import { messageOf } from "../../shared/errors.ts";
import { AddChoice } from "./AddChoice.tsx";
import { ChoiceRow } from "./ChoiceRow.tsx";
import { orderable, orderAfterDrop, orderAfterMove, placeOf } from "./choice-order.ts";
import { choiceKind, type OwnField } from "./fields.ts";
import { useCombineChoices, useReorderChoices } from "./use-change-field.ts";

/**
 * Puts the choices in the order the buttons ask for. Only the choices without a number are put in
 * order by hand (a number places the others), so every place here counts among those.
 */
function useChoiceOrder(field: OwnField) {
  const reorder = useReorderChoices();
  const send = (order: readonly string[]) => {
    reorder.mutate({
      params: { path: { field: field.id } },
      body: { version: field.version, order: [...order] },
    });
  };
  return {
    error: reorder.error,
    /** Choices can be dragged: not on a phone, where the buttons are the way. */
    canDrag: useLayout() !== "phone",
    /** The choice at `at` among those without a number goes up or down by one. */
    move: (at: number, by: number) => {
      send(orderAfterMove(field.choices, at, at + by));
    },
    /** A choice was dropped before the one at `before` among those without a number (their count for the end). */
    drop: (dropped: string, before: number) => {
      const order = orderAfterDrop(field.choices, dropped, before);
      if (order !== undefined) {
        send(order);
      }
    },
  };
}

/** What the list needs of the way the choices are put in order. */
interface Reordering {
  readonly canDrag: boolean;
  readonly move: (at: number, by: number) => void;
  readonly drop: (dropped: string, before: number) => void;
}

/** The list of the choices; the arrows and the places to drop count among the choices without a number. */
function ChoiceRows({
  field,
  order,
  onCombine,
}: {
  readonly field: OwnField;
  readonly order: Reordering;
  readonly onCombine: (choice: string, into: string) => void;
}) {
  return (
    <ul {...partOf(part.list)} aria-label={t("fields.choices")}>
      {field.choices.map((choice) => {
        const place = placeOf(field.choices, choice.id);
        return (
          <ChoiceRow
            key={choice.id}
            field={field}
            choice={choice}
            place={place}
            canDrag={order.canDrag}
            before={
              place === undefined ? null : (
                <DropSlot
                  kind={choiceKind}
                  enabled={order.canDrag}
                  onDrop={(dropped) => {
                    order.drop(dropped, place.at);
                  }}
                />
              )
            }
            others={field.choices.filter((other) => other.id !== choice.id && !other.retired)}
            onMove={(by) => {
              order.move(place?.at ?? 0, by);
            }}
            onCombine={(into) => {
              onCombine(choice.id, into);
            }}
          />
        );
      })}
    </ul>
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
  const rest = orderable(field.choices);
  return (
    <Card heading={t("fields.choices")}>
      <p {...partOf(part.muted)}>{t("fields.choices-help")}</p>
      {failure === null ? null : <Alert tone={alertTone.danger}>{messageOf(failure)}</Alert>}
      <ChoiceRows
        field={field}
        order={order}
        onCombine={(choice, into) => {
          combine.mutate({
            params: { path: { field: field.id, choice } },
            body: { version: field.version, into },
          });
        }}
      />
      <DropSlot
        kind={choiceKind}
        enabled={order.canDrag}
        onDrop={(dropped) => {
          order.drop(dropped, rest.length);
        }}
      />
      <AddChoice field={field} />
    </Card>
  );
}
