import { lineKey, type MapCard, type MapLine, type MapPicture } from "./layout.ts";

/**
 * Where the picture puts its cards, the names of its columns, its lines and their words, in pixels
 * from the corner of the picture. The sizes come from the theme file (`size-map-…`), so the
 * places follow the size of the text; nothing here depends on where the pointer is.
 */

/** The sizes of the picture, in pixels, as the theme file gives them. */
export interface MapSizes {
  readonly cardWidth: number;
  readonly cardHeight: number;
  /** The room between two columns, where the lines and their words run. */
  readonly gap: number;
  /** The room between two cards of a column. */
  readonly rowGap: number;
  /** The line of the names of the columns, above the first cards. */
  readonly head: number;
  /** The length of the head of an arrow. */
  readonly arrow: number;
}

/** A place in the picture, in pixels. */
export interface Point {
  readonly x: number;
  readonly y: number;
}

/** A line as it is drawn: its path, the head of its arrow, and where its word stands. */
export interface DrawnLine {
  readonly key: string;
  readonly line: MapLine;
  readonly path: string;
  /** The head of the arrow at the issue the line reads `backward` from; none for a line without direction. */
  readonly head: string | undefined;
  readonly label: Point;
}

/** Where the corner of a card stands. */
export function cardPlace(card: MapCard, sizes: MapSizes): Point {
  return {
    x: card.column * (sizes.cardWidth + sizes.gap),
    y: sizes.head + card.row * (sizes.cardHeight + sizes.rowGap),
  };
}

/** Where the name of a column stands. */
export function headPlace(column: number, sizes: MapSizes): Point {
  return { x: column * (sizes.cardWidth + sizes.gap), y: 0 };
}

/** How big the whole picture is. */
export function pictureSize(
  picture: MapPicture,
  sizes: MapSizes,
): { width: number; height: number } {
  const columns = Math.max(1, picture.columns.length);
  const rows = Math.max(1, ...picture.columns.map((cards) => cards.length));
  return {
    // The last column keeps the room of a line that goes round it to another card of the same column.
    width: columns * sizes.cardWidth + columns * sizes.gap,
    height: sizes.head + rows * sizes.cardHeight + (rows - 1) * sizes.rowGap,
  };
}

/** A number for a path, with as few digits as a pixel needs. */
function n(value: number): string {
  return String(Math.round(value * 10) / 10);
}

/** The head of an arrow whose tip is at `tip`, pointing to the right (`1`) or to the left (`-1`). */
function arrowHead(tip: Point, towards: 1 | -1, size: number): string {
  const back = tip.x - towards * size;
  const half = size / 2;
  return `M${n(tip.x)} ${n(tip.y)}L${n(back)} ${n(tip.y - half)}L${n(back)} ${n(tip.y + half)}Z`;
}

/** The point halfway along a cubic curve. */
function middleOf(from: Point, first: Point, second: Point, to: Point): Point {
  return {
    x: (from.x + 3 * first.x + 3 * second.x + to.x) / 8,
    y: (from.y + 3 * first.y + 3 * second.y + to.y) / 8,
  };
}

/**
 * A line between two cards: from the right edge of the card in the column on the left to the left
 * edge of the one on its right, or, between two cards of one column, round the right of that
 * column. Its arrow points at the issue that reads `backward` (the one that waits, the result, the
 * parent, the one kept), unless the kind has no direction.
 */
export function drawnLine(
  picture: MapPicture,
  line: MapLine,
  sizes: MapSizes,
  directed: boolean,
): DrawnLine | undefined {
  const from = picture.cards[line.from];
  const to = picture.cards[line.to];
  if (from === undefined || to === undefined) {
    return undefined;
  }
  const middle = sizes.cardHeight / 2;
  const at = (card: MapCard, side: "start" | "end"): Point => {
    const corner = cardPlace(card, sizes);
    return { x: corner.x + (side === "end" ? sizes.cardWidth : 0), y: corner.y + middle };
  };
  let start: Point;
  let end: Point;
  let first: Point;
  let second: Point;
  let towards: 1 | -1;
  if (from.column === to.column) {
    start = at(from, "end");
    end = at(to, "end");
    const reach = Math.min(sizes.gap * 0.8, sizes.gap / 4 + Math.abs(end.y - start.y) / 4);
    first = { x: start.x + reach, y: start.y };
    second = { x: end.x + reach, y: end.y };
    towards = -1;
  } else {
    const fromLeft = from.column < to.column;
    start = at(from, fromLeft ? "end" : "start");
    end = at(to, fromLeft ? "start" : "end");
    const bend = (end.x - start.x) / 2;
    first = { x: start.x + bend, y: start.y };
    second = { x: end.x - bend, y: end.y };
    towards = fromLeft ? 1 : -1;
  }
  return {
    key: lineKey(line),
    line,
    path: `M${n(start.x)} ${n(start.y)}C${n(first.x)} ${n(first.y)} ${n(second.x)} ${n(second.y)} ${n(end.x)} ${n(end.y)}`,
    head: directed ? arrowHead(end, towards, sizes.arrow) : undefined,
    label: middleOf(start, first, second, end),
  };
}
