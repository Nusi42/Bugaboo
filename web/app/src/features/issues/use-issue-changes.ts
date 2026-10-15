import { conflictOf, type Conflict } from "@bugaboo/api-client";
import { useState } from "react";

import { conflictSentence } from "../../shared/editing/index.ts";
import { personName, someoneName } from "../../shared/person.ts";
import { undoStore } from "../../shared/optimistic-undo.ts";
import { queryKeys } from "../../shared/query-keys.ts";
import { useOptimisticMutation, type OptimisticMutation } from "../../shared/use-optimistic.ts";
import { readAgain, sendChange, versionNow } from "./issue-requests.ts";
import {
  after,
  changeSentence,
  isDetail,
  valuesBefore,
  versionOf,
  type IssueEdit,
  type IssueValues,
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
  /** Who changed it first, as the reader may see them: for *Keep Ben's*. */
  readonly who: string;
  readonly keepTheirs: () => void;
  readonly useMine: () => void;
}

/** Who changed it first: a name, a role or *Someone*, as the reader may see them. */
function whoChanged(conflict: Conflict): string {
  return conflict.changed_by === null || conflict.changed_by === undefined
    ? someoneName("start")
    : personName(conflict.changed_by);
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

/** What the page of an issue gets for changing short values. */
export interface IssueChanges {
  /** Shows the change at once and sends it; *Undo* is on offer for 10 seconds. */
  readonly change: (values: IssueValues) => void;
  readonly saveState: OptimisticMutation<IssueEdit>["saveState"];
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
  const id = detail.issue.id ?? address;
  const mutation = useOptimisticMutation<IssueEdit, IssueDetail, void>({
    queryKey: queryKeys.issues.one(address),
    isCached: isDetail,
    apply: after,
    send: sendAndNote(id, { held: setHeld, savedAt: setSavedAt }),
  });
  const fallback = versionOf(detail.issue);
  const run = (values: IssueValues, version: number): void => {
    setHeld(null);
    mutation.change({ values, version });
  };
  return {
    change: (values) => {
      const before = valuesBefore(detail, values);
      run(values, versionNow(address, fallback));
      undoStore.offer({
        message: changeSentence(values),
        undo: () => {
          run(before, versionNow(address, fallback));
        },
      });
    },
    saveState: mutation.saveState,
    savedAt,
    problem: held === null ? mutation.problem : null,
    conflict:
      held === null
        ? null
        : {
            sentence: held.sentence,
            who: whoChanged(held.conflict),
            keepTheirs: () => {
              setHeld(null);
              readAgain(address);
            },
            useMine: () => {
              run(held.edit.values, held.conflict.version ?? versionNow(address, fallback));
            },
          },
  };
}
