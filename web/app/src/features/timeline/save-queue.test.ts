import { describe, expect, it } from "vitest";

import { createUndoStore } from "../../shared/optimistic-undo.ts";
import { manualClock, nextTick } from "../../shared/optimistic-testing.ts";
import { changeBook } from "./change-book.ts";
import type { KeptDates } from "./date-fields.ts";
import { keyRuns, runBook } from "./key-runs.ts";
import { saveQueue, type Step } from "./save-queue.ts";

/** A row of the timeline as the queue sees it: the ID of its issue. */
interface Row {
  readonly id: string;
}

const row: Row = { id: "web-1" };
const first = { start: "2027-03-03", end: "2027-03-05" };

/** A change of the bar from `before` to `after`, by whole dates. */
function step(before: KeptDates, after: KeptDates): Step {
  return {
    before,
    from: { first: before.start ?? "", last: before.end ?? "" },
    dates: after,
    days: { first: after.start ?? "", last: after.end ?? "" },
  };
}

/** The bars of the test are not drawn. */
function nothing(): void {
  // Nothing is on screen.
}

/** A queue whose server answers only when the test says so, and what it was sent. */
function queue() {
  const clock = manualClock();
  const offers = createUndoStore(clock.schedule);
  const sent: KeptDates[] = [];
  // What the server holds, as the page reads it again after each answer.
  let held: KeptDates = first;
  const answers: ((refused: string | null) => void)[] = [];
  const saving = saveQueue(changeBook(), {
    idOf: (each: Row) => each.id,
    send: (_row, dates) => {
      sent.push(dates);
      return new Promise((answer) => {
        answers.push(answer);
      });
    },
    text: (_row, _said, dates) => `saved ${dates.start ?? ""}`,
    show: nothing,
    kept: () => ({ dates: held, days: { first: held.start ?? "", last: held.end ?? "" } }),
    offers,
  });
  /** The server answers the oldest change it has not answered. */
  const answer = async (refused: string | null = null): Promise<void> => {
    const answered = sent[sent.length - answers.length];
    answers.shift()?.(refused);
    if (refused === null && answered !== undefined) {
      held = answered;
    }
    await nextTick();
  };
  const undo = (): boolean => {
    const offer = offers.list().find((each) => each.undo !== undefined);
    return offer !== undefined && offers.take(offer.id);
  };
  return { clock, offers, sent, saving, answer, undo };
}

const later = { start: "2027-03-10", end: "2027-03-12" };
const latest = { start: "2027-03-17", end: "2027-03-19" };

describe("the queue of the changes of a bar (PLAN-03)", () => {
  it("plan_03_an_undo_of_an_undo_brings_the_change_back", async () => {
    const { sent, saving, answer, undo } = queue();
    saving.save(row, step(first, later), { move: "set" });
    await answer();
    expect(undo()).toBe(true);
    await answer();
    expect(undo()).toBe(true);
    await answer();
    expect(sent).toEqual([later, first, later]);
  });

  it("plan_03_after_a_refusal_the_undo_of_a_later_change_puts_back_what_was_saved", async () => {
    const { offers, sent, saving, answer, undo } = queue();
    saving.save(row, step(first, later), { move: "set" });
    saving.save(row, step(later, latest), { move: "set" });
    await answer("refused");
    await answer();
    expect(offers.list().map((offer) => offer.message)).toEqual(["refused", "saved 2027-03-17"]);
    expect(undo()).toBe(true);
    await answer();
    expect(sent).toEqual([later, latest, first]);
  });

  it("plan_03_the_first_press_of_a_run_withdraws_the_older_undo", async () => {
    const { clock, offers, sent, saving, answer, undo } = queue();
    saving.save(row, step(first, later), { move: "set" });
    await answer();
    const keys = keyRuns(runBook<Row>(), saving, clock.schedule);
    keys.onKeyMove(row, "both", 1);
    expect(offers.list()).toEqual([]);
    expect(undo()).toBe(false);
    clock.advance(0);
    keys.onRunEnd(row.id);
    await answer();
    expect(sent).toEqual([later, { start: "2027-03-11", end: "2027-03-13" }]);
  });

  it("plan_03_a_change_answered_during_a_run_offers_no_undo", async () => {
    const { clock, offers, saving, answer, undo } = queue();
    saving.save(row, step(first, later), { move: "set" });
    const keys = keyRuns(runBook<Row>(), saving, clock.schedule);
    keys.onKeyMove(row, "both", 1);
    await answer();
    expect(offers.list().filter((each) => each.undo !== undefined)).toEqual([]);
    expect(undo()).toBe(false);
  });
});
