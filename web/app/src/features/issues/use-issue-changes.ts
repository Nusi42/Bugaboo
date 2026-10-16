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
      await sendChange(id, edit);
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

/** What the page of an issue gets for changing short values. */
export interface IssueChanges {
  /** Shows the change at once and sends it; *Undo* is on offer for 10 seconds. */
  readonly change: (values: IssueValues) => void;
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
  const run = (values: IssueValues, version: number): void => {
    setHeld(null);
    mutation.change({ values, version });
  };
  return {
    change: (values) => {
      const before = valuesBefore(detail, values);
      run(values, versionNow(address, detail.issue));
      const field = rowFieldOf(values);
      const offer = undoStore.offer({
        message: changeSentence(values),
        inline: field !== undefined,
        undo: () => {
          run(before, versionNow(address, detail.issue));
        },
      });
      rowChanged(field, offer);
    },
    saveState: mutation.saveState,
    lastRow,
    savedAt,
    problem: held === null ? mutation.problem : null,
    conflict:
      held === null
        ? null
        : {
            sentence: held.sentence,
            theirs: keepTheirsText(held.conflict),
            keepTheirs: () => {
              setHeld(null);
              readAgain(address);
            },
            useMine: () => {
              run(held.edit.values, held.conflict.version ?? versionNow(address, detail.issue));
            },
          },
  };
}
