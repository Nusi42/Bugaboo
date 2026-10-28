import { useEffect, useLayoutEffect, useRef, type ReactNode } from "react";

import { Icon } from "./Icon.tsx";
import type { ColumnResize } from "./use-column-resize.ts";
import { iconColumns, type ListColumn } from "./list-columns.ts";
import { listTemplate, type ListColumnSpec } from "./list-template.ts";
import { CellGroup, ColumnHeaderCell } from "./ListRoles.tsx";
import { partOf } from "./parts.ts";
import { buttonType, iconSize } from "./props.ts";
import { part } from "./tokens.ts";
import { variableOf } from "./variables.ts";

export interface ListFrameProps {
  readonly columns: readonly ListColumnSpec[];
  /** The characters of the longest ID the list draws, so the ID column holds it whole; none: the least. */
  readonly keyChars?: number;
  /** *Spacing: Compact*. */
  readonly compact: boolean;
  readonly children: ReactNode;
}

/**
 * Gives the rows what only the head knows. The rows are virtualized and have no width of their
 * own, so the head, which is not, is measured and its width is given to them (`list-width`, none
 * where there is no head: a phone has cards); and the room of the bar the rows have is kept at the
 * end of the head (`list-gutter`), so the columns stand over each other. Only the head, the box of
 * the rows and the first thing in it are measured, not what is drawn in it.
 */
function keepHeadAndRows(frame: HTMLElement): () => void {
  let box: HTMLElement | null = null;
  let head: HTMLElement | null = null;
  const written = { gutter: "", width: "" };
  const give = (
    name: "list-gutter" | "list-width",
    value: string,
    was: "gutter" | "width",
  ): void => {
    if (value !== written[was]) {
      written[was] = value;
      frame.style.setProperty(variableOf(name), value);
    }
  };
  const sync = (): void => {
    if (box !== null) {
      give("list-gutter", `${String(box.offsetWidth - box.clientWidth)}px`, "gutter");
    }
    // The width as drawn, not rounded to a whole pixel; none where there is no head.
    give(
      "list-width",
      head === null ? "0px" : `${String(head.getBoundingClientRect().width)}px`,
      "width",
    );
  };
  const watching = new ResizeObserver(sync);
  const attach = (): void => {
    const foundBox = frame.querySelector<HTMLElement>(`[data-part="${part.listScroll}"]`);
    const foundHead = frame.querySelector<HTMLElement>(`[data-part="${part.listHead}"]`);
    if (foundBox === box && foundHead === head) {
      return;
    }
    watching.disconnect();
    [box, head] = [foundBox, foundHead];
    for (const watched of [box, box?.firstElementChild, head]) {
      if (watched !== null && watched !== undefined) {
        watching.observe(watched);
      }
    }
    sync();
  };
  const changes = new MutationObserver(attach);
  changes.observe(frame, { childList: true, subtree: true });
  attach();
  return () => {
    watching.disconnect();
    changes.disconnect();
  };
}

/** How many characters the probe of the code font has: the measured width is divided by it. */
const PROBE_CHARACTERS = 10;

/**
 * Measures one character of the code font as the browser draws it (a probe in the code font, which
 * the code font of this computer may be any of) and gives it to the list as `size-code-char`, so
 * the ID column is as wide as the IDs really are. It measures again when the probe changes size: the
 * font arrives, or the text gets larger.
 */
function keepCharacterWidth(frame: HTMLElement): () => void {
  const probe = frame.querySelector<HTMLElement>(`[data-part="${part.listCharProbe}"]`);
  const measure = (): void => {
    const width = (probe?.getBoundingClientRect().width ?? 0) / PROBE_CHARACTERS;
    if (width > 0) {
      frame.style.setProperty(variableOf("size-code-char"), `${String(width)}px`);
    }
  };
  const watching = new ResizeObserver(measure);
  if (probe !== null) {
    watching.observe(probe);
  }
  measure();
  return () => {
    watching.disconnect();
  };
}

/** The box around the head and the rows of an issue list: it holds the columns they share. */
export function ListFrame({ columns, keyChars = 0, compact, children }: ListFrameProps) {
  const frame = useRef<HTMLDivElement>(null);
  const template = listTemplate(columns, keyChars);
  useLayoutEffect(() => {
    frame.current?.style.setProperty(variableOf("list-columns"), template);
  }, [template]);
  useEffect(() => (frame.current === null ? undefined : keepHeadAndRows(frame.current)), []);
  useLayoutEffect(
    () => (frame.current === null ? undefined : keepCharacterWidth(frame.current)),
    [],
  );
  return (
    <div ref={frame} {...partOf(part.listFrame, { variants: [compact && "compact"] })}>
      <span {...partOf(part.listCharProbe)} aria-hidden>
        {"0".repeat(PROBE_CHARACTERS)}
      </span>
      {children}
    </div>
  );
}

export interface ListCellProps {
  readonly column: ListColumn;
  /** The id of the head cell of its column: a screen reader hears the cell as a group named by it. */
  readonly headedBy?: string | undefined;
  /** The cell is a header of the head row (the box that ticks the rows shown). */
  readonly head?: boolean;
  readonly children?: ReactNode;
}

/** One cell of a row, in the column it belongs to (R9: the same things stand in the same places). */
export function ListCell({ column, headedBy, head = false, children }: ListCellProps) {
  const look = partOf(part.listRowCell, { variants: [column] });
  if (head) {
    return <ColumnHeaderCell look={look}>{children}</ColumnHeaderCell>;
  }
  if (headedBy === undefined) {
    return <div {...look}>{children}</div>;
  }
  return (
    <CellGroup look={look} labelledBy={headedBy}>
      {children}
    </CellGroup>
  );
}

export interface ListColumnHeadProps {
  readonly column: ListColumn;
  /** The name of the column. */
  readonly label: string;
  /** Names the button for screen readers: *Sort by Title*. */
  readonly sortLabel: string;
  /** Which way an arrow beside the name points; none when the list is not sorted by the column. */
  readonly arrow: "up" | "down" | null;
  /** The id the cells below it name their column by. */
  readonly id: string;
  readonly onSort: () => void;
  readonly resize: ColumnResize | null;
}

/** What a screen reader is told of the order a column sorts in. */
function sortOf(arrow: "up" | "down" | null): "ascending" | "descending" | "none" {
  if (arrow === null) {
    return "none";
  }
  return arrow === "up" ? "ascending" : "descending";
}

/** The name of a column above the rows: pressing it sorts by it, dragging its edge sets its width. */
export function ListColumnHead({
  column,
  label,
  sortLabel,
  arrow,
  id,
  onSort,
  resize,
}: ListColumnHeadProps) {
  return (
    <ColumnHeaderCell
      resize={resize === null || iconColumns.includes(column) ? null : resize}
      id={id}
      sort={sortOf(arrow)}
      label={label}
      look={partOf(part.listRowCell, { variants: [column] })}
    >
      <button
        type={buttonType.button}
        {...partOf(part.listSort)}
        aria-label={sortLabel}
        onClick={onSort}
      >
        {iconColumns.includes(column) ? null : (
          <span {...partOf(part.listSortLabel)} aria-hidden>
            {label}
          </span>
        )}
        {arrow === null ? null : <Icon name={arrow} size={iconSize.small} />}
      </button>
    </ColumnHeaderCell>
  );
}
