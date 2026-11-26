import { describe, expect, it } from "vitest";

import { cardPlace, drawnLine, pictureSize, type MapSizes } from "./geometry.ts";
import {
  chainOf,
  loopSteps,
  onChainOf,
  pictureOf,
  type ChainStep,
  type MapAnswer,
  type MapIssue,
  type MapLine,
} from "./layout.ts";

// The layout of the relation map (ISS-16, UX §4.15, ADR-0009 amendment 2026-11-24, item 2): the
// start on the left, a column for each step away, each column in the order of the answer.

/** An issue as the answer names it: its internal ID is its readable ID in lower case. */
function issueOf(key: string): MapIssue {
  return {
    id: key.toLowerCase(),
    readable_id: key,
    project: { id: "web", short_name: "WEB", name: "Website" },
    issue_type: { id: "task", name: "Task", icon: "square-check", retired: false },
    title: `Title of ${key}`,
  };
}

/** The words of a kind of relation, as the answer gives them. */
interface Kind {
  readonly relation_kind: string;
  readonly forward: string;
  readonly backward: string;
}

const blocks: Kind = { relation_kind: "k-blocks", forward: "blocks", backward: "is blocked by" };
const related: Kind = { relation_kind: "k-related", forward: "related to", backward: "related to" };

/** A line from one issue to another, by their readable IDs. */
function lineOf(from: string, to: string, kind: Kind = blocks): MapLine {
  return { from: from.toLowerCase(), to: to.toLowerCase(), ...kind };
}

/** The map of WEB-142 of UX §4.15, two steps away. */
function answerOfIssue(): MapAnswer {
  return {
    start: "web-142",
    issues: [
      { steps: 0, issue: issueOf("WEB-142") },
      { steps: 1, issue: issueOf("WEB-147") },
      { steps: 1, issue: issueOf("WEB-141") },
      { steps: 2, issue: issueOf("WEB-148") },
      { steps: 2, issue: issueOf("WEB-140") },
    ],
    lines: [
      lineOf("WEB-142", "WEB-147"),
      lineOf("WEB-141", "WEB-142", related),
      lineOf("WEB-148", "WEB-147"),
      lineOf("WEB-140", "WEB-141"),
    ],
    loops: [],
    truncated: false,
  };
}

const sizes: MapSizes = {
  cardWidth: 200,
  cardHeight: 60,
  gap: 160,
  rowGap: 16,
  head: 30,
  arrow: 8,
};

/** The picture as text: each column with its readable IDs, in order. */
function columnsOf(answer: MapAnswer): string[][] {
  return pictureOf(answer).columns.map((cards) => cards.map((card) => card.issue.readable_id));
}

/** Everything the screen draws from an answer: the places of the cards, the lines and the size. */
function drawingOf(answer: MapAnswer): unknown {
  const picture = pictureOf(answer);
  return {
    cards: Object.values(picture.cards).map((card) => [card.issue.id, cardPlace(card, sizes)]),
    lines: picture.lines.map((line) => drawnLine(picture, line, sizes, true)),
    size: pictureSize(picture, sizes),
  };
}

/** Steps as sentences of their own, for reading in a test. */
function told(steps: readonly ChainStep[] | undefined): string[] {
  return (steps ?? []).map(
    (step) => `${step.from.readable_id} ${step.word} ${step.to.readable_id}`,
  );
}

describe("the picture of an answer", () => {
  it("iss_16_the_same_answer_gives_the_same_picture", () => {
    const first = drawingOf(answerOfIssue());
    // The answer read again, in a new object, gives the same picture to the pixel.
    expect(drawingOf(answerOfIssue())).toStrictEqual(first);
    expect(drawingOf(structuredClone(answerOfIssue()))).toStrictEqual(first);
  });

  it("iss_16_the_start_stands_on_the_left_and_each_column_holds_one_step_in_the_order_of_the_answer", () => {
    expect(columnsOf(answerOfIssue())).toStrictEqual([
      ["WEB-142"],
      ["WEB-147", "WEB-141"],
      ["WEB-148", "WEB-140"],
    ]);
  });

  it("iss_16_the_order_inside_a_column_is_the_order_of_the_server", () => {
    const answer = answerOfIssue();
    const [start, first, second, ...rest] = answer.issues;
    const swapped: MapAnswer = {
      ...answer,
      issues: [start, second, first, ...rest].filter((each) => each !== undefined),
    };
    expect(columnsOf(swapped)[1]).toStrictEqual(["WEB-141", "WEB-147"]);
  });

  it("iss_16_the_map_of_a_search_starts_each_group_at_its_first_result", () => {
    const answer: MapAnswer = {
      issues: ["WEB-3", "WEB-1", "WEB-2", "WEB-9"].map((key) => ({ issue: issueOf(key) })),
      lines: [lineOf("WEB-1", "WEB-2"), lineOf("WEB-3", "WEB-1")],
      loops: [],
      truncated: false,
    };
    expect(columnsOf(answer)).toStrictEqual([["WEB-3", "WEB-9"], ["WEB-1"], ["WEB-2"]]);
  });
});

describe("chains and loops", () => {
  it("iss_16_a_chain_goes_back_to_the_start_one_relation_a_step", () => {
    const picture = pictureOf(answerOfIssue());
    expect(told(chainOf(picture, "web-148"))).toStrictEqual([
      "WEB-142 blocks WEB-147",
      "WEB-147 is blocked by WEB-148",
    ]);
    expect(chainOf(picture, "web-142")).toStrictEqual([]);
    const on = onChainOf(picture, "web-148");
    expect(on.issues.toSorted()).toStrictEqual(["web-142", "web-147", "web-148"]);
    expect(on.lines).toHaveLength(2);
  });

  it("iss_16_a_loop_is_read_as_relations_round_it", () => {
    const answer: MapAnswer = {
      start: "web-160",
      issues: [
        { steps: 0, issue: issueOf("WEB-160") },
        { steps: 1, issue: issueOf("WEB-161") },
      ],
      lines: [lineOf("WEB-160", "WEB-161"), lineOf("WEB-161", "WEB-160")],
      loops: [["web-160", "web-161"]],
      truncated: false,
    };
    expect(told(loopSteps(pictureOf(answer), ["web-160", "web-161"]))).toStrictEqual([
      "WEB-160 blocks WEB-161",
      "WEB-161 blocks WEB-160",
    ]);
  });
});

describe("the lines", () => {
  const picture = pictureOf(answerOfIssue());
  const [first, second] = picture.lines;
  const blocking = first === undefined ? undefined : drawnLine(picture, first, sizes, true);
  const alike = second === undefined ? undefined : drawnLine(picture, second, sizes, false);

  it("iss_16_a_line_runs_from_the_edge_of_its_card_and_its_word_stands_between_the_columns", () => {
    expect(blocking?.path).toMatch(/^M200 60C/u);
    expect(blocking?.label.x).toBe(280);
    expect(blocking?.head).toBeDefined();
  });

  it("iss_16_a_line_without_direction_has_no_arrow", () => {
    expect(alike?.path).toBeDefined();
    expect(alike?.head).toBeUndefined();
  });
});
