import { describe, expect, it } from "vitest";

import { type Movement, type Moved, nothingWrong, wrongIn } from "./movement-judge.ts";
import type { Box } from "./movement-record.ts";

/** A box of the given place and size. */
function box(x: number, y: number, width: number, height: number): Box {
  return { x, y, width, height };
}

/** A landmark that went from one box to another in one step. */
function moved(name: string, from: Box, last: Box): Moved {
  return { name, from, to: last, frame: 1, steps: 1, last, turns: 0 };
}

/** A measure where only these landmarks moved and nothing else happened. */
function movementOf(parts: readonly Moved[], appeared: Movement["appeared"] = []): Movement {
  return {
    moved: parts,
    appeared,
    shifts: [],
    vanished: [],
    greyShapes: false,
    scrolled: [],
    frames: 10,
  };
}

const tags = moved("tags", box(900, 600, 300, 50), box(900, 600, 300, 90));
const labels = (to: number) => moved("labels", box(900, 660, 300, 44), box(900, to, 300, 44));

describe("ux_01 the measure ties every push to what grew above it", () => {
  it("passes a part pushed by exactly what grew above it in its column", () => {
    const found = wrongIn(movementOf([tags, labels(700)]), {
      grows: ["tags"],
      follows: ["labels"],
    });
    expect(found).toEqual(nothingWrong);
  });

  it("fails a part pushed 200 px when nothing grew above it", () => {
    const pushed = moved("history", box(400, 900, 480, 300), box(400, 1100, 480, 300));
    const found = wrongIn(movementOf([pushed]), { follows: ["history"] });
    expect(found.moved).toEqual([pushed]);
  });

  it("fails a part pushed by more than grew above it", () => {
    const found = wrongIn(movementOf([tags, labels(720)]), {
      grows: ["tags"],
      follows: ["labels"],
    });
    expect(found.moved.map((part) => part.name)).toEqual(["labels"]);
  });

  it("does not count what grew in another column", () => {
    const beside = moved("main", box(400, 900, 480, 300), box(400, 940, 480, 300));
    const found = wrongIn(movementOf([tags, beside]), { grows: ["tags"], follows: ["main"] });
    expect(found.moved.map((part) => part.name)).toEqual(["main"]);
  });
});

describe("ux_01 the measure holds what is named nowhere, what came in and what grows", () => {
  it("fails a part that moved and is named nowhere", () => {
    const head = moved("head", box(400, 100, 480, 40), box(400, 101, 480, 40));
    expect(wrongIn(movementOf([head]), {}).moved).toEqual([head]);
  });

  it("passes rows pushed by exactly the row that came in above them", () => {
    const row = moved("row 2", box(300, 400, 900, 44), box(300, 444, 900, 44));
    const found = wrongIn(
      movementOf([row], [{ name: "the new row", last: box(300, 400, 900, 44) }]),
      { follows: ["row 2"] },
    );
    expect(found).toEqual(nothingWrong);
  });

  it("passes a part that grows upwards from its bottom edge, and fails one whose both edges move", () => {
    const footer = moved("footer", box(0, 630, 360, 110), box(0, 580, 360, 160));
    expect(wrongIn(movementOf([footer]), { grows: ["footer"] })).toEqual(nothingWrong);
    const loose = moved("footer", box(0, 630, 360, 110), box(0, 600, 360, 100));
    expect(wrongIn(movementOf([loose]), { grows: ["footer"] }).moved).toEqual([loose]);
  });

  it("fails a part that is pushed back and forth even when it ends where it should", () => {
    const twice = { ...labels(700), steps: 2 };
    const found = wrongIn(movementOf([tags, twice]), { grows: ["tags"], follows: ["labels"] });
    expect(found.moved).toEqual([twice]);
  });

  it("fails a part that changes and shrinks and grows back, even when it ends as it should", () => {
    const bouncing = { ...tags, turns: 2 };
    expect(wrongIn(movementOf([bouncing]), { grows: ["tags"] }).moved).toEqual([bouncing]);
  });
});
