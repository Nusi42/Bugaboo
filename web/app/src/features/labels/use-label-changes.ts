import { type ChangeStatus } from "@bugaboo/ui";
import { t } from "@bugaboo/messages";
import { useState } from "react";

import { saveLabels } from "../../shared/save-labels.ts";
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

/** What the row of the last change says: where it stands, and how to take it back. */
interface Last {
  readonly row: LabelRow;
  readonly state: "saving" | "saved" | "not-saved";
  readonly savedAt: string | null;
  readonly undo?: () => void;
}

/** A change of a row: what sends it, and what takes it back once it is made. */
type Sending = () => Promise<IssueLabelsAndTags>;
type TakingBack = (made: IssueLabelsAndTags) => Sending | undefined;

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
  readonly addLabel: (label: string) => Promise<boolean>;
  readonly takeLabelOff: (label: Chip) => void;
}

/** The moment of a change, for *Saved at …*. */
function momentNow(): string {
  return new Date().toISOString();
}

/** What a row says of the last change, when the last change was in that row. */
function statusOf(last: Last | undefined, row: LabelRow): ChangeStatus | undefined {
  if (last?.row !== row) {
    return undefined;
  }
  return {
    state: last.state,
    labels: saveLabels(last.savedAt),
    ...(last.undo === undefined ? {} : { undo: { label: t("common.undo"), onUndo: last.undo } }),
  };
}

/**
 * Makes one change of a row of one issue and keeps what the row says about it. When `back` says
 * how to take the change back, the row offers *Undo*, which makes that change and then only says
 * *Saved*.
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
      const inverse = back?.(made);
      const undo =
        inverse === undefined
          ? {}
          : {
              undo: () => {
                void change(row, inverse);
              },
            };
      setLast({ row, state: "saved", savedAt: momentNow(), ...undo });
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
 * *Undo* in its own row (UX §4.4, R7).
 */
export function useLabelChanges(issue: string): LabelChanges {
  const { last, error, change } = useChange(issue);
  return {
    status: (row) => statusOf(last, row),
    error,
    pending: last?.state === "saving",
    addTag: (name, has) =>
      change(
        "tags",
        () => putTag(issue, name),
        (made) => {
          // A tag that was on the issue already changes nothing, so there is nothing to take back.
          const added = made.tags?.find((tag) => !has.some((each) => each.id === tag.id));
          return added === undefined ? undefined : () => takeTagOff(issue, added.id);
        },
      ),
    takeTagOff: (tag) => {
      void change(
        "tags",
        () => takeTagOff(issue, tag.id),
        () => () => putTag(issue, tag.name),
      );
    },
    addLabel: (label) =>
      change(
        "labels",
        () => putLabel(issue, label),
        () => () => takeLabelOff(issue, label),
      ),
    takeLabelOff: (label) => {
      void change(
        "labels",
        () => takeLabelOff(issue, label.id),
        () => () => putLabel(issue, label.id),
      );
    },
  };
}
