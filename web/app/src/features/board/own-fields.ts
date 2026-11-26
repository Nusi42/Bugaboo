import { timelineGroupable, type SearchFieldsAnswer } from "../search/index.ts";
import type { OwnColumns } from "../issues/index.ts";
import type { OwnOf } from "./CardFields.tsx";

/** A choice of a box in *Customize*: a field by its word and its name. */
interface FieldChoice {
  readonly value: string;
  readonly label: string;
}

/** The fields the columns and the rows of a board can come from, by their words and names. */
export function choicesOf(fields: SearchFieldsAnswer, own: OwnColumns) {
  const named = (word: string): FieldChoice => ({ value: word, label: fields.names[word] ?? word });
  const rows = fields.timelineGroups.filter((word) => timelineGroupable(fields)(word));
  return {
    columns: [
      named("status group"),
      ...Object.keys(own)
        .filter((word) => own[word]?.kind === "pick-one")
        .map((word) => named(word)),
    ],
    rows: rows.map((word) => named(word)),
  };
}

/** The own fields the person sees, found by their ID: what a card shows of them. */
export function ownByIdOf(own: OwnColumns, names: SearchFieldsAnswer["names"]): OwnOf {
  return (id) => {
    const word = Object.keys(own).find((each) => own[each]?.id === id);
    const column = word === undefined ? undefined : own[word];
    return word === undefined || column === undefined
      ? undefined
      : { column, name: names[word] ?? word };
  };
}
