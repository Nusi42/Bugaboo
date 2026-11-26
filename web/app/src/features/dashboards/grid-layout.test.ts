import { describe, expect, it } from "vitest";

import { testPacker as pack } from "../../shared/wording-testing.ts";
import {
  droppedAt,
  grownBy,
  placeAtEnd,
  readingOrder,
  resizedTo,
  steppedBy,
  swappedInOrder,
  type GridPlace,
} from "./grid-layout.ts";

/** A place, written short. */
function at(column: number, row: number, width: number, height: number): GridPlace {
  return { column, row, width, height };
}

/** Two boxes side by side on the top row, and a wide one under them. */
const three = [at(0, 0, 6, 4), at(6, 0, 6, 4), at(0, 4, 12, 4)] as const;

/** Two boxes in one column, and one beside them. */
const column = [at(0, 0, 6, 4), at(0, 4, 6, 4), at(6, 0, 6, 4)] as const;

describe("the places of the grid of a dashboard", () => {
  it("cfg_03_reads_the_boxes_row_by_row_from_the_start", () => {
    expect(readingOrder([at(6, 0, 6, 2), at(0, 2, 6, 2), at(0, 0, 6, 2)])).toEqual([2, 0, 1]);
  });

  it("cfg_03_a_box_dropped_below_another_in_its_column_takes_its_place", () => {
    expect(droppedAt(column, 0, { column: 0, row: 5 }, pack)).toEqual([
      at(0, 4, 6, 4),
      at(0, 0, 6, 4),
      at(6, 0, 6, 4),
    ]);
  });

  it("cfg_03_a_box_dropped_where_packing_lifts_it_ends_where_packing_puts_it", () => {
    // Below the wide box it would leave the top left free: packing lifts it back.
    expect(droppedAt(three, 0, { column: 0, row: 5 }, pack)).toEqual([...three]);
    // Into another column it goes under the box there.
    expect(droppedAt(column, 1, { column: 6, row: 4 }, pack)).toEqual([
      at(0, 0, 6, 4),
      at(6, 4, 6, 4),
      at(6, 0, 6, 4),
    ]);
  });

  it("cfg_03_a_column_is_kept_inside_the_grid", () => {
    expect(droppedAt([at(0, 0, 6, 4)], 0, { column: 9, row: 0 }, pack)).toEqual([at(6, 0, 6, 4)]);
  });

  it("cfg_03_a_wider_box_pushes_the_box_beside_it_down", () => {
    expect(resizedTo(three, 0, { width: 8, height: 4 }, pack)).toEqual([
      at(0, 0, 8, 4),
      at(6, 4, 6, 4),
      at(0, 8, 12, 4),
    ]);
  });

  it("cfg_03_a_size_stays_within_the_least_and_the_most", () => {
    expect(resizedTo([at(0, 0, 6, 4)], 0, { width: 1, height: 20 }, pack)).toEqual([
      at(0, 0, 3, 8),
    ]);
  });
});

describe("the keys of the grid of a dashboard", () => {
  it("cfg_03_move_with_the_keys_steps_a_column_or_past_a_neighbor", () => {
    expect(steppedBy([at(0, 0, 6, 4)], 0, "right", pack)).toEqual([at(1, 0, 6, 4)]);
    expect(steppedBy([at(0, 0, 6, 4)], 0, "left", pack)).toEqual([at(0, 0, 6, 4)]);
    const swapped = [at(0, 4, 6, 4), at(0, 0, 6, 4), at(6, 0, 6, 4)];
    expect(steppedBy(column, 0, "down", pack)).toEqual(swapped);
    expect(steppedBy(column, 1, "up", pack)).toEqual(swapped);
    // The top box has nothing above it.
    expect(steppedBy(three, 0, "up", pack)).toEqual([...three]);
  });

  it("cfg_03_resize_with_the_keys_does_what_the_corner_does", () => {
    expect(grownBy(three, 0, "right", pack)).toEqual(
      resizedTo(three, 0, { width: 7, height: 4 }, pack),
    );
    expect(grownBy(three, 0, "down", pack)).toEqual(
      resizedTo(three, 0, { width: 6, height: 5 }, pack),
    );
  });

  it("cfg_03_move_up_and_down_on_a_phone_swap_neighbors_in_the_reading_order", () => {
    expect(readingOrder(swappedInOrder(three, 1, true, pack))).toEqual([1, 0, 2]);
    expect(readingOrder(swappedInOrder(three, 0, false, pack))).toEqual([1, 0, 2]);
    expect(swappedInOrder(three, 2, false, pack)).toEqual([...three]);
  });

  it("cfg_03_a_new_box_goes_under_every_box", () => {
    expect(placeAtEnd(three, { width: 6, height: 2 })).toEqual(at(0, 8, 6, 2));
    expect(placeAtEnd([], { width: 6, height: 2 })).toEqual(at(0, 0, 6, 2));
  });
});
