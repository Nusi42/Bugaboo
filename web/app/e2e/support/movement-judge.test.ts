import { describe, expect, it } from "vitest";

import { judged, type Movement, type Moved, nothingWrong, wrongIn } from "./movement-judge.ts";
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

  it("passes a changed part that gets wider from its start edge, as a button with a longer word", () => {
    const button = moved("status", box(400, 240, 110, 36), box(400, 240, 140, 36));
    expect(wrongIn(movementOf([button]), { grows: ["status"] })).toEqual(nothingWrong);
  });

  it("fails a changed part that gets narrower, as a title whose words wrap anew", () => {
    const title = moved("title", box(400, 120, 600, 60), box(400, 120, 560, 60));
    expect(wrongIn(movementOf([title]), { grows: ["title"] }).moved).toEqual([title]);
  });
});

describe("ux_01 the measure lets what holds a row that came in grow by that row", () => {
  const row = { name: "new row", last: box(900, 700, 300, 44) };

  it("passes a holder that grew by exactly the row that came into it", () => {
    const rows = moved("rows", box(900, 400, 300, 600), box(900, 400, 300, 644));
    expect(wrongIn(movementOf([rows], [row]), {})).toEqual(nothingWrong);
  });

  it("fails a holder that grew by more than the row that came into it", () => {
    const rows = moved("rows", box(900, 400, 300, 600), box(900, 400, 300, 700));
    expect(wrongIn(movementOf([rows], [row]), {}).moved).toEqual([rows]);
  });

  it("fails a holder that moved, or grew and shrank, while the row came in", () => {
    const pushed = moved("rows", box(900, 400, 300, 600), box(900, 420, 300, 644));
    const bounced = {
      ...moved("rows", box(900, 400, 300, 600), box(900, 400, 300, 644)),
      turns: 2,
    };
    expect(wrongIn(movementOf([pushed, bounced], [row]), {}).moved).toEqual([pushed, bounced]);
  });
});

describe("ux_01 the measure lets a part the action moves move sideways, once", () => {
  it("passes a part the action moves, once, and fails one that goes there and back", () => {
    const bar = moved("bar", box(400, 300, 120, 24), box(440, 300, 120, 24));
    expect(wrongIn(movementOf([bar]), { moves: ["bar"] })).toEqual(nothingWrong);
    const back = { ...bar, turns: 1 };
    expect(wrongIn(movementOf([back]), { moves: ["bar"] }).moved).toEqual([back]);
  });

  it("fails a moved part that also goes down or changes its size", () => {
    const down = moved("bar", box(400, 300, 120, 24), box(440, 330, 120, 24));
    const wider = moved("bar", box(400, 300, 120, 24), box(440, 300, 160, 24));
    expect(wrongIn(movementOf([down, wider]), { moves: ["bar"] }).moved).toEqual([down, wider]);
  });
});

describe("ux_01 the measure tells a row that left the view from one drawn afresh", () => {
  const shown = box(0, 0, 300, 44);
  /** Three frames of one landmark: there, then gone, with or without the same thing drawn afresh. */
  const frames = (redrawn: boolean) => ({
    boxes: [[shown], [null], [null]],
    redrawn: [[false], [redrawn], [false]],
    scrolls: ["page:0", "page:0", "page:0"],
    grey: [false, false, false],
    view: 800,
  });

  it("does not count a row of a list gone from the view as gone", () => {
    expect(judged([["row", true]], frames(false)).vanished).toEqual([]);
  });

  it("counts a row drawn afresh as gone", () => {
    expect(judged([["row", true]], frames(true)).vanished).toEqual(["row"]);
  });

  it("counts any other part that is no longer there as gone", () => {
    expect(judged([["head", false]], frames(false)).vanished).toEqual(["head"]);
  });
});

describe("ux_01 the measure judges what is in view, and lets the browser keep it still", () => {
  const view = 800;
  /** Something above the view grew by 52 px, and the scroll of the page kept what is in view still. */
  const above = moved("head", box(0, -300, 300, 100), box(0, -352, 300, 100));
  const scroll = { part: "app-layout.main", from: "600/0", to: "652/0" };
  const withScroll = (parts: readonly Moved[]): Movement => ({
    ...movementOf(parts),
    scrolled: [scroll],
    view,
  });

  it("passes a scroll that kept what is in view still while something above it grew", () => {
    expect(wrongIn(withScroll([above]), {})).toEqual(nothingWrong);
  });

  it("fails a part in view that moved, scroll or no scroll", () => {
    const row = moved("row", box(0, 200, 300, 44), box(0, 230, 300, 44));
    expect(wrongIn(withScroll([above, row]), {}).moved).toEqual([row]);
  });

  it("fails a scroll when nothing above the view changed", () => {
    expect(wrongIn(withScroll([]), {}).scrolled).toEqual([scroll]);
  });
});
