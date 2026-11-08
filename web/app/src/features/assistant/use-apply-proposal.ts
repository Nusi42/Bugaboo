import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { useState } from "react";

import { messageOf } from "../../shared/errors.ts";
import { undoStore } from "../../shared/optimistic-undo.ts";
import { refreshHistory, refreshIssues } from "../../shared/query-keys.ts";
import { useUndoOffers } from "../../shared/use-optimistic.ts";
import { useApply, type AppliedProposal, type Proposal, type ProposedChange } from "./assistant.ts";
import type { CardChange, CardState } from "./ProposalCard.tsx";
import { canTakeBack, takeBack, undoProblem } from "./undo-change.ts";

/** What a member of a tool's input is, as text; none when it is not text. */
export function inputText(input: unknown, member: string): string | undefined {
  if (typeof input !== "object" || input === null) {
    return undefined;
  }
  const value: unknown = Reflect.get(input, member);
  return typeof value === "string" ? value : undefined;
}

/** A change of the API as the card draws it. */
export function cardChange(
  change: ProposedChange,
  words: readonly Schema<"FieldChangeBody">[],
): CardChange {
  return {
    field: change.field,
    before: change.before,
    after: change.after,
    words: words.find((each) => each.field === change.field),
  };
}

/** Where the card stands: from what the person decided and what the server answered. */
function stateOf(
  dropped: boolean,
  undone: boolean,
  failure: string | undefined,
  applying: "idle" | "pending" | "success" | "error",
  error: unknown,
): CardState {
  if (dropped) {
    return { kind: "discarded" };
  }
  if (applying === "pending") {
    return { kind: "saving" };
  }
  if (applying === "success") {
    if (undone) {
      return { kind: "undone" };
    }
    return failure === undefined ? { kind: "saved" } : { kind: "saved", problem: failure };
  }
  return applying === "error" ? { kind: "refused", problem: messageOf(error) } : { kind: "open" };
}

/** Puts *Undo* on offer for ten seconds. It is undone only once the server did it; a refusal is said on the card in its own words. */
function offerUndo(
  issue: string | undefined,
  applied: AppliedProposal,
  saved: readonly CardChange[],
  then: { readonly done: () => void; readonly failed: (problem: string) => void },
): number {
  return undoStore.offer({
    message: t("assistant.card.undone"),
    inline: true,
    undo: () => {
      takeBack(issue, applied, saved).then(then.done, (error: unknown) => {
        then.failed(undoProblem(error));
      });
    },
  });
}

/** What the card shows of a proposal, and what its buttons do. */
export interface ProposalControl {
  readonly state: CardState;
  readonly changes: readonly CardChange[];
  readonly apply: () => void;
  readonly discard: () => void;
  /** The way back, for as long as it is on offer. */
  readonly undo: (() => void) | undefined;
}

/**
 * Applying a proposal: *Apply* saves exactly what the card shows, as the person, and offers *Undo* for
 * the ten seconds every undo is on offer for; *Discard* drops the proposal and saves nothing.
 */
export function useApplyProposal(proposal: Proposal): ProposalControl {
  const applying = useApply();
  const [dropped, setDropped] = useState(false);
  const [undone, setUndone] = useState(false);
  const [problem, setProblem] = useState<string>();
  const [offer, setOffer] = useState<number>();
  const live = useUndoOffers().some((each) => each.id === offer);
  const issue = inputText(proposal.input, "issue");
  const offerBack = (applied: AppliedProposal): void => {
    const saved = applied.changes.map((change) => cardChange(change, applied.words));
    if (canTakeBack(issue, applied, saved)) {
      setOffer(
        offerUndo(issue, applied, saved, {
          done: () => {
            setUndone(true);
          },
          failed: setProblem,
        }),
      );
    }
  };
  const shown: Pick<Proposal, "changes" | "words"> = applying.data ?? proposal;
  return {
    state: stateOf(dropped, undone, problem, applying.status, applying.error),
    changes: shown.changes.map((change) => cardChange(change, shown.words)),
    apply: () => {
      applying.mutate(
        { body: { reference: proposal.reference, tool: proposal.tool, input: proposal.input } },
        {
          onSuccess: (applied: AppliedProposal) => {
            refreshIssues();
            refreshHistory();
            offerBack(applied);
          },
        },
      );
    },
    discard: () => {
      setDropped(true);
    },
    undo:
      live && offer !== undefined
        ? () => {
            undoStore.take(offer);
          }
        : undefined,
  };
}
