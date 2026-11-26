import { boxHeightLeast, boxHeightMost, boxWidthLeast, gridColumns } from "@bugaboo/api-client";
import type { BoxPlace, Packer } from "@bugaboo/wasm";

/**
 * Where a box stands on the grid of a dashboard and how big it is, in columns and rows from 0 (ADR-0050, decision 3).
 * The places are always packed upwards, by the packing of the server (`bb_core::dashboard::pack`, through the
 * WebAssembly module): a free row above a box cannot be, so a move that would leave one ends where packing puts it.
 */
export type GridPlace = BoxPlace;

/** Whether two places share a cell. */
function overlaps(one: GridPlace, other: GridPlace): boolean {
  return (
    one.column < other.column + other.width &&
    other.column < one.column + one.width &&
    one.row < other.row + other.height &&
    other.row < one.row + one.height
  );
}

/** A number kept between two bounds. */
function within(value: number, least: number, most: number): number {
  return Math.min(Math.max(value, least), most);
}

/** The first column of a box of `width` kept inside the grid. */
function columnInGrid(column: number, width: number): number {
  return within(column, 0, gridColumns - width);
}

/** A place with where it stands in the list of places. */
interface Indexed {
  readonly place: GridPlace;
  readonly at: number;
}

/** The places, each with its index. */
function indexed(places: readonly GridPlace[]): readonly Indexed[] {
  return places.map((place, at): Indexed => ({ place, at }));
}

/**
 * The boxes in the order they are read: row by row, and in a row from the start (the order of a phone, where they stand
 * in one column, and the order the server keeps them in).
 */
export function readingOrder(places: readonly GridPlace[]): readonly number[] {
  return indexed(places)
    .toSorted(
      (one: Indexed, other: Indexed) =>
        one.place.row - other.place.row ||
        one.place.column - other.place.column ||
        one.at - other.at,
    )
    .map((each: Indexed) => each.at);
}

/**
 * The places of the boxes laid out in `order`, then packed: each first goes, in its column and size, to the highest row
 * that is not above the box before it and meets no box laid out before it, so the order is kept; packing then closes
 * every free row above a box, as the server does when the change is saved.
 */
function laidOut(
  places: readonly GridPlace[],
  order: readonly number[],
  pack: Packer,
): GridPlace[] {
  const proposed = [...places];
  const settled: GridPlace[] = [];
  let lowest = 0;
  for (const at of order) {
    const place = places[at];
    if (place === undefined) {
      continue;
    }
    let row = lowest;
    while (settled.some((other) => overlaps(other, { ...place, row }))) {
      row += 1;
    }
    const placed = { ...place, row };
    settled.push(placed);
    proposed[at] = placed;
    lowest = row;
  }
  return [...pack(proposed)];
}

/** Where a box goes among the others in the reading order: just before or just after one of them, or last. */
export type Beside = { readonly before: number } | { readonly after: number } | "last";

/**
 * The places after the box `at` was put beside another (or last) in the reading order, in its `shape` (column and
 * size, kept inside the grid and the bounds of a box): the boxes are laid out again in that order, so the others make
 * room once, where it lands. The answer is in the order of `places`.
 */
export function placedBeside(
  places: readonly GridPlace[],
  at: number,
  beside: Beside,
  pack: Packer,
  shape: Partial<GridPlace> = {},
): GridPlace[] {
  const place = places[at];
  if (place === undefined) {
    return [...places];
  }
  const width = within(shape.width ?? place.width, boxWidthLeast, gridColumns);
  const height = within(shape.height ?? place.height, boxHeightLeast, boxHeightMost);
  const moved = {
    ...place,
    width,
    height,
    column: columnInGrid(shape.column ?? place.column, width),
  };
  const others = readingOrder(places).filter((each) => each !== at);
  const order =
    beside === "last"
      ? [...others, at]
      : "before" in beside
        ? others.flatMap((each) => (each === beside.before ? [at, each] : [each]))
        : others.flatMap((each) => (each === beside.after ? [each, at] : [each]));
  return laidOut(
    places.map((each, index) => (index === at ? moved : each)),
    order,
    pack,
  );
}

/** A cell of the grid. */
export interface Cell {
  readonly column: number;
  readonly row: number;
}

/**
 * The places after the box `at` was let go at a cell (a drag): it goes before the first other box that stands at that
 * row from that column on, or lower, and the others make room once.
 */
export function droppedAt(
  places: readonly GridPlace[],
  at: number,
  cell: Cell,
  pack: Packer,
): GridPlace[] {
  const next = readingOrder(places).find((each) => {
    const other = places[each];
    return (
      each !== at &&
      other !== undefined &&
      (other.row > cell.row || (other.row === cell.row && other.column >= cell.column))
    );
  });
  return placedBeside(places, at, next === undefined ? "last" : { before: next }, pack, {
    column: cell.column,
  });
}

/** A size of a box, in columns and rows. */
export interface BoxSize {
  readonly width: number;
  readonly height: number;
}

/** The places after the box `at` was given another size (its corner dragged, or *Resize* with the arrow keys). */
export function resizedTo(
  places: readonly GridPlace[],
  at: number,
  size: BoxSize,
  pack: Packer,
): GridPlace[] {
  const order = readingOrder(places);
  const before = order[order.indexOf(at) + 1];
  return placedBeside(places, at, before === undefined ? "last" : { before }, pack, size);
}

/** The ways a box moves or grows by the arrow keys. */
export type Step = "up" | "down" | "left" | "right";

/** The nearest box above (or below) the box `at` that shares a column with it, if there is one. */
function nextInColumn(places: readonly GridPlace[], at: number, up: boolean): number | undefined {
  const place = places[at];
  if (place === undefined) {
    return undefined;
  }
  const near = indexed(places)
    .filter(
      ({ place: other, at: index }: Indexed) =>
        index !== at &&
        other.column < place.column + place.width &&
        place.column < other.column + other.width &&
        (up ? other.row < place.row : other.row > place.row),
    )
    .toSorted((one: Indexed, two: Indexed) =>
      up ? two.place.row - one.place.row : one.place.row - two.place.row,
    );
  return near[0]?.at;
}

/**
 * The places after one step of *Move* with an arrow key: sideways by a column, up before the box above it, down after
 * the box below it. A step off the grid, or one that packing takes back, changes nothing.
 */
export function steppedBy(
  places: readonly GridPlace[],
  at: number,
  step: Step,
  pack: Packer,
): GridPlace[] {
  const place = places[at];
  if (place === undefined) {
    return [...places];
  }
  if (step === "left" || step === "right") {
    const column = columnInGrid(place.column + (step === "left" ? -1 : 1), place.width);
    return droppedAt(places, at, { column, row: place.row }, pack);
  }
  const other = nextInColumn(places, at, step === "up");
  if (other === undefined) {
    return [...places];
  }
  return placedBeside(places, at, step === "up" ? { before: other } : { after: other }, pack);
}

/** The places after one step of *Resize* with an arrow key: a column narrower or wider, a row lower or higher. */
export function grownBy(
  places: readonly GridPlace[],
  at: number,
  step: Step,
  pack: Packer,
): GridPlace[] {
  const place = places[at];
  if (place === undefined) {
    return [...places];
  }
  const width = place.width + (step === "left" ? -1 : step === "right" ? 1 : 0);
  const height = place.height + (step === "up" ? -1 : step === "down" ? 1 : 0);
  return resizedTo(places, at, { width, height }, pack);
}

/**
 * The places after *Move up* or *Move down* on a phone, where the boxes stand in one column in the reading order: the
 * box takes the column of its neighbor in that order and goes before (or after) it.
 */
export function swappedInOrder(
  places: readonly GridPlace[],
  at: number,
  up: boolean,
  pack: Packer,
): GridPlace[] {
  const order = readingOrder(places);
  const other = order[order.indexOf(at) + (up ? -1 : 1)];
  const neighbor = other === undefined ? undefined : places[other];
  if (other === undefined || neighbor === undefined) {
    return [...places];
  }
  return placedBeside(places, at, up ? { before: other } : { after: other }, pack, {
    column: neighbor.column,
  });
}

/** Whether two lists of places are the same. */
export function samePlaces(one: readonly GridPlace[], other: readonly GridPlace[]): boolean {
  return (
    one.length === other.length &&
    one.every((place, at) => {
      const that = other[at];
      return (
        that !== undefined &&
        place.column === that.column &&
        place.row === that.row &&
        place.width === that.width &&
        place.height === that.height
      );
    })
  );
}

/** Where a new box of a size goes: at the start of the row under every box. */
export function placeAtEnd(places: readonly GridPlace[], size: BoxSize): GridPlace {
  const row = Math.max(0, ...places.map((place) => place.row + place.height));
  return { column: 0, row, width: size.width, height: size.height };
}

/** The numbers of a place as the grid of the theme takes them: its lines counted from 1, and its spans. */
export function gridPlaceOf(place: GridPlace) {
  return {
    "box-column": place.column + 1,
    "box-row": place.row + 1,
    "box-width": place.width,
    "box-height": place.height,
  };
}
