import { t } from "@bugaboo/messages";
import { Checkbox, ListCell, ListColumnHead, part, partOf, type ListColumn } from "@bugaboo/ui";

import { cellOf } from "../../shared/list-columns.ts";
import type { ListSort } from "./list-sort.ts";

const selectColumn = "select" satisfies ListColumn;

/** The box in the head that ticks the rows shown. */
export interface HeadTick {
  readonly isSelected: boolean;
  readonly onChange: (selected: boolean) => void;
}

export interface ListHeadRowProps {
  /** The search words of the columns, in order. */
  readonly words: readonly string[];
  /** The name of each field, by its search word. */
  readonly names: Readonly<Record<string, string>>;
  /** How the list is ordered now. */
  readonly sort: ListSort | null;
  readonly tick: HeadTick | null;
  /** A press on the name of a column. */
  readonly onSort: (word: string) => void;
  /** The width of a column while it is dragged, and when the drag ends. */
  readonly onResize: (word: string, width: number, final: boolean) => void;
}

/** What a press on the name of a column does, told to a screen reader. */
function sortLabelOf(name: string, arrow: "up" | "down" | null): string {
  if (arrow === null) {
    return t("issues.sort-by", { column: name });
  }
  return t(arrow === "up" ? "issues.sorted-up" : "issues.sorted-down", { column: name });
}

/**
 * The names of the columns above the rows (UX section 4.2): pressing a name sorts by it, which
 * writes `sort by` into the search; dragging the edge of a column, or moving it with the arrow
 * keys, sets its width. It has the columns of the rows, so the cells line up.
 */
export function ListHeadRow({ words, names, sort, tick, onSort, onResize }: ListHeadRowProps) {
  return (
    <div {...partOf(part.listHead)}>
      <div {...partOf(part.listRow, { variants: ["head", "grid"] })}>
        {tick === null ? null : (
          <ListCell column={selectColumn}>
            <Checkbox
              hideLabel
              label={t("issues.select-shown")}
              isSelected={tick.isSelected}
              onChange={tick.onChange}
            />
          </ListCell>
        )}
        {words.map((word) => {
          const name = names[word] ?? word;
          const arrow = sort?.word === word ? sort.arrow : null;
          return (
            <ListColumnHead
              key={word}
              column={cellOf(word)}
              label={name}
              sortLabel={sortLabelOf(name, arrow)}
              arrow={arrow}
              onSort={() => {
                onSort(word);
              }}
              resize={{
                label: t("issues.column-width", { column: name }),
                onResize: (width) => {
                  onResize(word, width, false);
                },
                onResized: (width) => {
                  onResize(word, width, true);
                },
              }}
            />
          );
        })}
      </div>
    </div>
  );
}
