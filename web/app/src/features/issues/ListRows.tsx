import { t } from "@bugaboo/messages";
import { Checkbox, ListCell, part, partOf, rowStates, type ListColumn } from "@bugaboo/ui";
import { Collection, GridListItem, GridListLoadMoreItem } from "react-aria-components";

import type { FoundLine } from "./found-issues.ts";
import { hrefOf } from "./issue-show.ts";
import { rowCells } from "./list-cells.tsx";
import { useListContext } from "./list-context.ts";

const selectColumn = "select" satisfies ListColumn;

/** A row with the key it has in its group. */
interface KeyedLine {
  readonly id: string;
  readonly line: FoundLine;
}

/** One issue of the list (UX §4.2): a box to tick, then a cell for each column. */
export function IssueItem({ id, line }: { readonly id: string; readonly line: FoundLine }) {
  const context = useListContext();
  const selected = context.selection.has(line.id);
  const current = line.key === context.open;
  const project = line.project?.short_name ?? context.project ?? "";
  const cells = rowCells(context.words, line, {
    href: hrefOf(context.openIn, project, line.key),
    now: context.now,
    own: context.own,
    current,
  });
  return (
    <GridListItem
      id={id}
      textValue={line.title}
      {...partOf(part.listRow, {
        variants: [context.phone ? "phone" : "grid"],
        states: [...rowStates(line), selected && "selected", current && "current"],
      })}
    >
      {context.ticks ? (
        <ListCell column={selectColumn}>
          <Checkbox
            hideLabel
            label={t("issues.select-issue", { key: line.key })}
            isSelected={selected}
            onChange={(next) => {
              context.selection.set(line.id, next);
            }}
          />
        </ListCell>
      ) : null}
      {cells.map(({ word, cell, content }) => (
        <ListCell key={word} column={cell}>
          {content}
        </ListCell>
      ))}
    </GridListItem>
  );
}

export interface RowsOfProps {
  /** The prefix that makes the key of a row different from the same issue in another group. */
  readonly group: string;
  readonly lines: readonly FoundLine[];
  readonly hasMore: boolean;
  readonly isLoading: boolean;
  /** The next page, or the first one of a group that has not been asked for yet. */
  readonly onMore: () => void;
}

/** The rows read so far, and the place that asks for more when it comes into view. */
export function RowsOf({ group, lines, hasMore, isLoading, onMore }: RowsOfProps) {
  return (
    <>
      <Collection items={lines.map((line): KeyedLine => ({ id: `${group}${line.id}`, line }))}>
        {(keyed: KeyedLine) => <IssueItem id={keyed.id} line={keyed.line} />}
      </Collection>
      {hasMore ? <GridListLoadMoreItem onLoadMore={onMore} isLoading={isLoading} /> : null}
    </>
  );
}
