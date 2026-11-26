import { t } from "@bugaboo/messages";
import { describe, expect, it } from "vitest";

import type { BoardCard, BoardColumn } from "./board-data.ts";
import { choiceOf, columnName, mayGo, moveOf, reasonOf } from "./card-moves.ts";

const column = (key: string | null, group: BoardColumn["status_group"] = null): BoardColumn => ({
  key,
  status_group: group,
  count: 0,
  limit: null,
  over: false,
});

const card: BoardCard = {
  fields: [],
  issue: {
    readable_id: "WEB-139",
    status: { key: "open", name: "Open", status_group: "not-started" },
  },
  moves: [
    { column: 1, needs: [], refused: null, to: "in-progress" },
    { column: 2, needs: [], refused: "no-such-status", to: null },
    { column: 3, needs: [], refused: "no-such-change", to: null },
    { column: 4, needs: [], refused: "cannot-change", to: null },
  ],
};

const words = (at: number) =>
  reasonOf(card, moveOf(card, at), { column: column("Testing"), field: "Severity" });

describe("where a card may go (PLAN-06)", () => {
  it("names a column by its status, its status group's words or what it lacks", () => {
    expect(columnName(column("In progress"))).toBe("In progress");
    expect(columnName(column("In progress", "in-progress"))).toBe(t("status-group.in-progress"));
    expect(columnName(column(null))).toBe(t("board.no-value"));
  });

  it("takes the judgment of the server for the move of a column", () => {
    expect(mayGo(moveOf(card, 1))).toBe(true);
    expect(mayGo(moveOf(card, 2))).toBe(false);
    expect(mayGo(moveOf(card, 0))).toBe(false);
  });

  it("says why in the words of the board for each reason of the server", () => {
    expect(words(1)).toBeUndefined();
    expect(words(2)).toBe(t("board.reason-no-status", { key: "WEB-139", column: "Testing" }));
    expect(words(3)).toBe(
      t("board.reason-no-change", { key: "WEB-139", from: "Open", column: "Testing" }),
    );
    expect(words(4)).toBe(t("board.reason-cannot-change", { key: "WEB-139", field: "Severity" }));
    expect(words(0)).toBe(t("board.reason-no-status-read", { key: "WEB-139" }));
  });

  it("finds the choice of a column of a choice field, and none for the column of no value", () => {
    const field = { id: "f1", kind: "pick-one", choices: { c1: "Minor", c2: "Major" } } as const;
    expect(choiceOf(field, column("Major"))).toBe("c2");
    expect(choiceOf(field, column(null))).toBeNull();
    expect(choiceOf(field, column("Gone"))).toBeUndefined();
  });
});
