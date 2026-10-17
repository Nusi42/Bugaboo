import { t } from "@bugaboo/messages";
import { type ChangeStatus, type SaveState } from "@bugaboo/ui";
import { useState } from "react";

import { undoStore } from "../../shared/optimistic-undo.ts";
import { useUndoOffers } from "../../shared/use-optimistic.ts";
import { rowStatusOf } from "../issues/index.ts";
import {
  afterLabelsAndTags,
  putLabel,
  putTag,
  takeLabelOff,
  takeTagOff,
  type Chip,
  type IssueLabelsAndTags,
} from "./labels.ts";

/** The two rows of the Details that this area draws. */
export type LabelRow = "tags" | "labels";

/** A tag or label as a change names it: the ID and the name. */
export type Named = Pick<Chip, "id" | "name">;

/** The last change of the rows: where it was made, how it stands, and the offer to take it back. */
interface Last {
  readonly row: LabelRow;
  readonly state: Exclude<SaveState, "idle">;
  readonly savedAt: string | null;
  readonly offer?: number;
}

/** A change of a row: what sends it, and what takes it back once it is made. */
type Sending = () => Promise<IssueLabelsAndTags>;
interface TakingBack {
  /** The sentence of the offer of *Undo*. */
  readonly message: string;
  readonly send: (made: IssueLabelsAndTags) => Sending | undefined;
}

/** What the rows of *Tags* and *Labels* do: change, with *Saved* and *Undo* in the row that changed. */
export interface LabelChanges {
  /** What the row says about the last change made in it (R7); nothing for a row that did not change. */
  readonly status: (row: LabelRow) => ChangeStatus | undefined;
  /** What went wrong with the last change; `null` when nothing did. */
  readonly error: unknown;
  /** Whether a change is on its way. */
  readonly pending: boolean;
  /** Adding says whether the change was made, so the box can be emptied; taking off needs no answer. */
  readonly addTag: (name: string, has: readonly Chip[]) => Promise<boolean>;
  readonly takeTagOff: (tag: Chip) => void;
  readonly addLabel: (label: Named) => Promise<boolean>;
  readonly takeLabelOff: (label: Chip) => void;
}

/** The moment of a change, for *Saved at …*. */
function momentNow(): string {
  return new Date().toISOString();
}

/**
 * Makes one change of a row of one issue and keeps what the row says about it. When `back` says
 * how to take the change back, it is offered through the store of undo, inline in the row, like
 * any other change of the Details; *Undo* makes that change and then only says *Saved*.
 */
function useChange(issue: string) {
  const [last, setLast] = useState<Last | undefined>();
  const [error, setError] = useState<unknown>(null);
  const change = async (row: LabelRow, send: Sending, back?: TakingBack): Promise<boolean> => {
    setError(null);
    setLast({ row, state: "saving", savedAt: null });
    try {
      const made = await send();
      afterLabelsAndTags(issue, made);
      const inverse = back?.send(made);
      const offer =
        back === undefined || inverse === undefined
          ? {}
          : {
              offer: undoStore.offer({
                message: back.message,
                inline: true,
                undo: () => {
                  void change(row, inverse);
                },
              }),
            };
      setLast({ row, state: "saved", savedAt: momentNow(), ...offer });
      return true;
    } catch (failure) {
      setError(failure);
      setLast({ row, state: "not-saved", savedAt: null });
      return false;
    }
  };
  return { last, error, change };
}

/**
 * The changes of the tags and labels of one issue. Each is made in place and says *Saved* with
 * *Undo* in its own row (UX §4.4, R7), by the helper every row of the Details uses.
 */
export function useLabelChanges(issue: string): LabelChanges {
  const { last, error, change } = useChange(issue);
  const offers = useUndoOffers();
  return {
    status: (row) =>
      rowStatusOf(
        {
          lastRow:
            last === undefined
              ? null
              : { field: last.row, undo: offers.find((each) => each.id === last.offer) },
          saveState: last?.state ?? "idle",
          savedAt: last?.savedAt ?? null,
        },
        row,
      ),
    error,
    pending: last?.state === "saving",
    addTag: (name, has) =>
      change("tags", () => putTag(issue, name), {
        message: t("labels.tag-added", { name }),
        send: (made) => {
          // A tag that was on the issue already changes nothing, so there is nothing to take back.
          const added = made.tags?.find((tag) => !has.some((each) => each.id === tag.id));
          return added === undefined ? undefined : () => takeTagOff(issue, added.id);
        },
      }),
    takeTagOff: (tag) => {
      void change("tags", () => takeTagOff(issue, tag.id), {
        message: t("labels.tag-taken-off", { name: tag.name }),
        send: () => () => putTag(issue, tag.name),
      });
    },
    addLabel: (label) =>
      change("labels", () => putLabel(issue, label.id), {
        message: t("labels.label-added", { name: label.name }),
        send: () => () => takeLabelOff(issue, label.id),
      }),
    takeLabelOff: (label) => {
      void change("labels", () => takeLabelOff(issue, label.id), {
        message: t("labels.label-taken-off", { name: label.name }),
        send: () => () => putLabel(issue, label.id),
      });
    },
  };
}
