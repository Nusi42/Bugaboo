import { conflictOf, type Conflict } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { useState } from "react";

import { changedByName, conflictSentence } from "../../shared/editing/index.ts";
import { undoStore, type UndoOffer } from "../../shared/optimistic-undo.ts";
import { queryKeys } from "../../shared/query-keys.ts";
import {
  useOptimisticMutation,
  useUndoOffers,
  type OptimisticMutation,
} from "../../shared/use-optimistic.ts";
import { readAgain, sendChange, versionNow } from "./issue-requests.ts";
import {
  after,
  changeSentence,
  isDetail,
  rowFieldOf,
  valuesBefore,
  type IssueEdit,
  type IssueValues,
  type RowField,
} from "./issue-values.ts";
import type { IssueDetail } from "./slots.ts";

export type { IssueEdit, IssueValues } from "./issue-values.ts";

/** A change that somebody else's change came before, with the sentence that says so. */
interface Held {
  readonly edit: IssueEdit;
  readonly conflict: Conflict;
  readonly sentence: string;
}

/** Somebody changed it first: the sentence, and the two answers. */
export interface IssueConflict {
  readonly sentence: string;
  /** The words of *Keep Ben's change*. */
  readonly theirs: string;
  readonly keepTheirs: () => void;
  readonly useMine: () => void;
}

/** *Keep Ben's change*; for a person who is not shown by name, *Keep their change*. */
function keepTheirsText(conflict: Conflict): string {
  return conflict.changed_by?.shown === "name"
    ? t("issues.keep-theirs", { name: changedByName(conflict) })
    : t("issues.keep-theirs-unnamed");
}

/** What a send tells the page: a change that came second, and the time of a save. */
interface Notes {
  readonly held: (held: Held) => void;
  readonly savedAt: (moment: string) => void;
}

/** The send of a change that writes down a conflict or the time of the save. */
function sendAndNote(id: string, notes: Notes): (edit: IssueEdit) => Promise<void> {
  return async (edit) => {
    try {
      const version = await sendChange(id, edit);
      if (version !== undefined) {
        edit.landed?.(version);
      }
    } catch (error) {
      const conflict = conflictOf(error);
      if (conflict !== undefined) {
        notes.held({ edit, conflict, sentence: conflictSentence(conflict, Date.now()) });
      }
      throw error;
    }
    notes.savedAt(new Date().toISOString());
  };
}

/** The row of the Details that changed last, and the offer of *Undo* for that change, while there is one. */
export interface LastRow {
  readonly field: RowField;
  readonly undo: UndoOffer | undefined;
}

/**
 * The row of the Details that changed last, with the offer of *Undo* while it is still on offer
 * (the store drops it after 10 seconds).
 */
function useLastRow(): {
  readonly lastRow: LastRow | null;
  readonly rowChanged: (field: RowField | undefined, offer: number) => void;
} {
  const [row, setRow] = useState<{ readonly field: RowField; readonly offer: number } | null>(null);
  const offers = useUndoOffers();
  return {
    lastRow:
      row === null
        ? null
        : { field: row.field, undo: offers.find((each) => each.id === row.offer) },
    rowChanged: (field, offer) => {
      setRow(field === undefined ? null : { field, offer });
    },
  };
}

/** What the answers to a change that came second do. */
interface ConflictWays {
  /** Takes the change back and reads the issue again. */
  readonly drop: () => void;
  /** Sends the values again, on this version when the server named one. */
  readonly again: (values: IssueValues, version: number | null | undefined) => void;
}

/** The two answers to a change that somebody else made first: *Keep Ben's* and *Use mine*. */
function conflictAnswers(held: Held, ways: ConflictWays): IssueConflict {
  return {
    sentence: held.sentence,
    theirs: keepTheirsText(held.conflict),
    keepTheirs: ways.drop,
    useMine: () => {
      ways.again(held.edit.values, held.conflict.version);
    },
  };
}

/** What a change that was asked for first needs to be sent and taken back. */
interface Asked {
  readonly detail: IssueDetail;
  readonly address: string;
  readonly id: string;
  readonly values: IssueValues;
  /** Sends values on a version, as *Undo* does. */
  readonly run: (values: IssueValues, version: number) => void;
}

/** Sends a change and waits for the answer; once it is made, *Undo* is on offer. */
async function askAndOffer({ detail, address, id, values, run }: Asked): Promise<void> {
  const before = valuesBefore(detail, values);
  const made = await sendChange(id, { values, version: versionNow(address, detail.issue) });
  readAgain(address);
  undoStore.offer({
    message: changeSentence(values),
    inline: false,
    undo: () => {
      run(before, made ?? versionNow(address, detail.issue));
    },
  });
}

/** What the answers to a change that came second do, for the issue on this page. */
function waysOf(
  setHeld: (held: Held | null) => void,
  address: string,
  detail: IssueDetail,
  run: Asked["run"],
): ConflictWays {
  return {
    drop: () => {
      setHeld(null);
      readAgain(address);
    },
    again: (values, version) => {
      run(values, version ?? versionNow(address, detail.issue));
    },
  };
}

/** What the page of an issue gets for changing short values. */
export interface IssueChanges {
  /** Shows the change at once and sends it; *Undo* is on offer for 10 seconds. */
  readonly change: (values: IssueValues) => void;
  /**
   * Sends a change that asked for something first (*Ask for*) and waits for the answer, so the
   * window that asked can name the field the server says is still empty. Rejects with the refusal;
   * *Undo* is on offer for 10 seconds like for any change.
   */
  readonly ask: (values: IssueValues) => Promise<void>;
  readonly saveState: OptimisticMutation<IssueEdit>["saveState"];
  /**
   * The row of the Details that the last change was made in, with the offer to take it back while
   * it lasts; nothing when the last change belongs to no row (the title, the status).
   */
  readonly lastRow: LastRow | null;
  /** When the last change was saved, as a moment in time. */
  readonly savedAt: string | null;
  /** The plain message when the server refused for another reason than a change made first. */
  readonly problem: string | null;
  readonly conflict: IssueConflict | null;
}

/**
 * Changing short values of an issue in place (UX §4.4, UX-06): the change shows at once and goes to
 * the server on the version that was looked at, *Saving…* turns into *Saved* with the time, and
 * *Undo* is on offer for 10 seconds. When somebody changed the issue first, the change is taken back
 * from the screen and the page offers *Keep Ben's* and *Use mine*, built from the structured answer.
 */
export function useIssueChanges(detail: IssueDetail, address: string): IssueChanges {
  const [held, setHeld] = useState<Held | null>(null);
  const [savedAt, setSavedAt] = useState<string | null>(null);
  const { lastRow, rowChanged } = useLastRow();
  const id = detail.issue.id ?? address;
  const mutation = useOptimisticMutation<IssueEdit, IssueDetail, void>({
    queryKey: queryKeys.issues.one(address),
    isCached: isDetail,
    apply: after,
    send: sendAndNote(id, { held: setHeld, savedAt: setSavedAt }),
  });
  const run = (values: IssueValues, version: number, landed?: (version: number) => void): void => {
    setHeld(null);
    mutation.change({ values, version, ...(landed === undefined ? {} : { landed }) });
  };
  return {
    change: (values) => {
      const before = valuesBefore(detail, values);
      const made: { version?: number } = {};
      run(values, versionNow(address, detail.issue), (version) => {
        made.version = version;
      });
      const field = rowFieldOf(values);
      const offer = undoStore.offer({
        message: changeSentence(values),
        inline: field !== undefined,
        undo: () => {
          run(before, made.version ?? versionNow(address, detail.issue));
        },
      });
      rowChanged(field, offer);
    },
    ask: (values) => askAndOffer({ detail, address, id, values, run }),
    saveState: mutation.saveState,
    lastRow,
    savedAt,
    problem: held === null ? mutation.problem : null,
    conflict: held === null ? null : conflictAnswers(held, waysOf(setHeld, address, detail, run)),
  };
}
