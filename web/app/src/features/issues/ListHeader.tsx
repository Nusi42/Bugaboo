import type { Dispatch, SetStateAction } from "react";

import type { ListView } from "../../shared/list-view.ts";
import { historyStep, type HistoryStep } from "../../shared/search-address.ts";
import type { SearchFieldsAnswer } from "../search/index.ts";
import { useListContext } from "./list-context.ts";
import { ListHeadRow } from "./ListHeadRow.tsx";
import { nextSort, sortOf } from "./list-sort.ts";

export interface ListHeaderProps {
  readonly view: ListView;
  readonly fields: SearchFieldsAnswer;
  readonly shown: readonly string[];
  readonly onView: (view: ListView) => void;
  readonly onText: (text: string, step: HistoryStep) => void;
  /** The width of the column that is dragged right now. */
  readonly onDragged: Dispatch<SetStateAction<Readonly<Record<string, number>>>>;
}

/** The head of the list wired to the search and to the view (UX §4.2): sorting, widths and *Tick the issues shown*. */
export function ListHeader({ view, fields, shown, onView, onText, onDragged }: ListHeaderProps) {
  const { words, ticks, selection, search, language } = useListContext();
  const kindOf = (word: string) =>
    fields.fields.fields.find((field) => field.word === word)?.kind ?? "text";
  return (
    <ListHeadRow
      words={words}
      names={fields.names}
      sort={sortOf(language, search.text)}
      tick={
        ticks
          ? {
              isSelected: shown.length > 0 && shown.every((id) => selection.has(id)),
              onChange: (on) => {
                selection.replace(
                  on
                    ? [...new Set([...selection.ids, ...shown])]
                    : selection.ids.filter((id) => !shown.includes(id)),
                );
              },
            }
          : null
      }
      onSort={(word) => {
        onText(nextSort(language, search.text, word, kindOf(word)), historyStep.push);
      }}
      onResize={(word, width, final) => {
        if (final) {
          onDragged({});
          onView({ ...view, widths: { ...view.widths, [word]: width } });
        } else {
          onDragged({ [word]: width });
        }
      }}
    />
  );
}
