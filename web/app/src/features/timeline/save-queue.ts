import type { UndoStore } from "../../shared/optimistic-undo.ts";
import type { Before, ChangeBook } from "./change-book.ts";
import type { KeptDates } from "./date-fields.ts";
import type { Said } from "./dates.ts";
import type { BarDays } from "./lines.ts";
import { moved, shownWhileMoving, type Move } from "./moves.ts";

/** Where the dates of an issue stand on this screen: what it keeps and what its bar shows. */
export interface Standing {
  readonly dates: KeptDates;
  readonly days: BarDays;
}

/** A step of a bar from where it stands: the dates and days before, the dates and days after. */
export interface Step extends Standing {
  readonly before: KeptDates;
  /** The days the bar showed before; none for an issue that had no bar. */
  readonly from: BarDays | undefined;
}

/** The step a move makes from where a bar stands. */
export function stepFrom(now: Standing, move: Move, days: number): Step {
  return {
    before: now.dates,
    from: now.days,
    dates: moved(now.dates, now.days, move, days),
    days: shownWhileMoving(now.days, move, days),
  };
}

/** The change of *Set dates…*; none without any date, which would send the issue to the tray. */
export function setTo(before: Before, dates: KeptDates): Step | undefined {
  const [first, last] = [dates.start ?? dates.end, dates.end ?? dates.start];
  return first === null || last === null
    ? undefined
    : { before: before.dates, from: before.days, dates, days: { first, last } };
}

/** The change of an issue dropped on a day: it starts there, and keeps a due date that comes after. */
export function placeOn(before: Before, day: string): Step {
  const kept = before.dates.end;
  const end = kept !== null && kept >= day ? kept : null;
  return {
    before: before.dates,
    from: before.days,
    dates: { start: day, end },
    days: { first: day, last: end ?? day },
  };
}

/** What the queue needs of the screen: the issue of a row, the server, the words, the bars. */
export interface QueueWays<Row> {
  readonly idOf: (row: Row) => string;
  /** Saves the dates; answers the server's reason when it refused. */
  readonly send: (row: Row, dates: KeptDates) => Promise<string | null>;
  readonly text: (row: Row, said: Said, dates: KeptDates) => string;
  /** Shows days on the bar of an issue; none shows the server's again. */
  readonly show: (id: string, days?: BarDays) => void;
  /** Where the issue of a row stands when no change of it is on its way. */
  readonly kept: (row: Row) => Standing | undefined;
  readonly offers: Pick<UndoStore, "offer" | "withdraw">;
}

/** A change has ended: the bar shows the server's days when it was the last and no run holds it. */
function ended<Row>(book: ChangeBook, ways: QueueWays<Row>, id: string): void {
  const now = book.get(id);
  if (now !== undefined && now.count > 1) {
    book.put(id, { ...now, count: now.count - 1 });
    return;
  }
  book.forget(id);
  if (!book.holds(id)) {
    ways.show(id);
  }
}

/** A change that the server answered, and its number among the changes of its issue. */
interface Answered<Row> {
  readonly row: Row;
  readonly step: Step;
  readonly said: Said;
  readonly newest: number;
  readonly refused: string | null;
}

/** Withdraws the *Undo* on offer for an issue: a newer change of it starts. */
function withdrawOffer<Row>(book: ChangeBook, ways: QueueWays<Row>, id: string): void {
  const offered = book.offerOf(id);
  if (offered !== undefined) {
    ways.offers.withdraw(offered);
  }
}

/**
 * The step back of a change: it starts where the issue stands now, which is where the change left
 * it, so an *Undo* of this *Undo* brings the change back.
 */
function stepBack<Row>(
  book: ChangeBook,
  ways: QueueWays<Row>,
  change: Answered<Row>,
  back: Before,
): Step {
  const now = book.get(ways.idOf(change.row)) ?? change.step;
  return {
    before: now.dates,
    from: now.days,
    dates: back.dates,
    days: back.days ?? change.step.days,
  };
}

/**
 * The changes of dates on their way (PLAN-03): each is shown on its bar at once and saved one after
 * the other. A new change starts from the dates the last one saved or is saving, never from the
 * page read before. *Undo* is a change like the others, in the same queue; the *Undo* on offer is
 * withdrawn when a newer change of the same issue starts, so it never puts back old dates over new
 * ones. A bar shows the server's days again only when its last change has ended and no run of keys
 * holds it.
 *
 * *Undo* puts back what the server held before the change, not what the screen thought it would
 * hold: when the server refuses a change, the changes after it in the queue start from the dates
 * saved before it, so the *Undo* of a later change never puts back the dates of a refused one.
 */
export function saveQueue<Row>(book: ChangeBook, ways: QueueWays<Row>) {
  const answered = (change: Answered<Row>): void => {
    const id = ways.idOf(change.row);
    const now = book.get(id);
    const back = now?.saved ?? { dates: change.step.before, days: change.step.from };
    if (change.refused !== null) {
      ways.offers.offer({ message: change.refused });
    } else if (now !== undefined) {
      book.put(id, { ...now, saved: { dates: change.step.dates, days: change.step.days } });
      if (now.newest === change.newest && !book.holds(id)) {
        const message = ways.text(change.row, change.said, change.step.dates);
        const undo = (): void => {
          save(change.row, stepBack(book, ways, change, back), { move: "set" });
        };
        book.setOffer(id, ways.offers.offer({ message, undo }));
      }
    }
    ended(book, ways, id);
  };
  function save(row: Row, step: Step, said: Said): void {
    const id = ways.idOf(row);
    withdrawOffer(book, ways, id);
    const before = book.get(id);
    const newest = (before?.newest ?? 0) + 1;
    const saved = before?.saved ?? { dates: step.before, days: step.from };
    const count = (before?.count ?? 0) + 1;
    book.put(id, { dates: step.dates, days: step.days, count, newest, saved });
    ways.show(id, step.days);
    void ways.send(row, step.dates).then((refused) => {
      answered({ row, step, said, newest, refused });
    });
  }
  return {
    idOf: ways.idOf,
    show: ways.show,
    save,
    withdraw: (id: string): void => {
      withdrawOffer(book, ways, id);
    },
    /** Where the issue of a row stands: the change on its way, or what the page read. */
    standing: (row: Row): Standing | undefined => book.get(ways.idOf(row)) ?? ways.kept(row),
    /** Shows the days of the issue's change on its way, or the server's when none is. */
    settle: (id: string): void => {
      ways.show(id, book.get(id)?.days);
    },
    hold: book.hold,
    release: book.release,
  };
}

/** What the screen saves through. */
export type SaveQueue<Row> = ReturnType<typeof saveQueue<Row>>;
