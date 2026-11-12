import { t } from "@bugaboo/messages";
import {
  Checkbox,
  headId,
  ListCell,
  part,
  partOf,
  rowStates,
  swipeSide,
  type ListColumn,
} from "@bugaboo/ui";
import { Collection, GridListItem, GridListLoadMoreItem } from "react-aria-components";

import type { FoundLine } from "./found-issues.ts";
import { hrefOf } from "./issue-show.ts";
import { rowCells } from "./list-cells.tsx";
import { useRowSwipe } from "../../shared/use-row-swipe.tsx";
import { IssueRowActions } from "./IssueRowActions.tsx";
import { useListContext } from "./list-context.ts";

const selectColumn = "select" satisfies ListColumn;

/** The sides of a card of a phone a swipe opens: *Follow* at the start, the windows of other areas at the end. */
const rowSides = [swipeSide.start, swipeSide.end] as const;

/** A row with the key it has in its group. */
interface KeyedLine {
  readonly id: string;
  readonly line: FoundLine;
}

/** The box to tick of a row. */
function TickCell({ line, selected }: { readonly line: FoundLine; readonly selected: boolean }) {
  const { selection } = useListContext();
  return (
    <ListCell column={selectColumn}>
      <Checkbox
        hideLabel
        label={t("issues.select-issue", { key: line.key })}
        isSelected={selected}
        onChange={(next) => {
          selection.set(line.id, next);
        }}
      />
    </ListCell>
  );
}

/** One issue of the list (UX §4.2): a box to tick, then a cell for each column. */
export function IssueItem({ id, line }: { readonly id: string; readonly line: FoundLine }) {
  const context = useListContext();
  const selected = context.selection.has(line.id);
  const current = line.key === context.open;
  const project = line.project?.short_name ?? context.project ?? "";
  const href = hrefOf(context.openIn, context.state, project, line.key);
  const cells = rowCells(context.words, line, {
    href,
    now: context.now,
    own: context.own,
    current,
  });
  const spoken = `${line.key} ${line.title}`;
  const swipe = useRowSwipe(rowSides);
  return (
    <GridListItem
      id={id}
      href={href}
      textValue={spoken}
      {...swipe.rowProps}
      {...partOf(part.listRow, {
        variants: [context.phone ? "phone" : "grid", context.phone && !context.ticks && "plain"],
        states: [...rowStates(line), selected && "selected", current && "current"],
      })}
    >
      {context.ticks ? <TickCell line={line} selected={selected} /> : null}
      {cells.map(({ word, cell, content }) => (
        <ListCell
          key={word}
          column={cell}
          headedBy={context.phone ? undefined : headId(context.listId, word)}
        >
          {content}
        </ListCell>
      ))}
      {context.phone ? (
        <IssueRowActions issue={{ id: line.id, key: line.key, project }} swipe={swipe} />
      ) : null}
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
