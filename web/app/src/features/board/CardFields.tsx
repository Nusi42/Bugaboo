import { part, partOf } from "@bugaboo/ui";
import type { ReactNode } from "react";
import { VisuallyHidden } from "react-aria-components";

import { fieldNameOf } from "../../shared/field-names.ts";
import type { BoardCard } from "./board-data.ts";
import { valueOf, type OwnOf } from "./card-values.tsx";

export type { OwnField, OwnOf } from "./card-values.tsx";

export interface CardFieldsProps {
  readonly card: BoardCard;
  readonly own: OwnOf;
}

/** The fields of the layout *Card* that the issue has a value for, each beside its name for screen readers only. */
export function CardFields({ card, own }: CardFieldsProps) {
  const lines = card.fields.flatMap((code) => {
    const value = valueOf(code, card.issue, own);
    return value === undefined || value === ""
      ? []
      : [<Line key={code} name={nameOf(code, own)} value={value} />];
  });
  return lines.length === 0 ? null : <div {...partOf(part.boardCardFields)}>{lines}</div>;
}

/** What a field of a card is called: an own field by its name, a field of the catalogue by its message. */
function nameOf(code: string, own: OwnOf): string {
  return own(code)?.name ?? fieldNameOf({ code });
}

/** One field of a card: its value, and its name for screen readers only. */
function Line({ name, value }: { readonly name: string; readonly value: ReactNode }) {
  return (
    <span>
      <VisuallyHidden>{name} </VisuallyHidden>
      {value}
    </span>
  );
}
