import { t } from "@bugaboo/messages";
import { useState } from "react";

import { messageOf } from "../../shared/errors.ts";
import { undoStore } from "../../shared/optimistic-undo.ts";
import { refreshHistory, refreshIssues } from "../../shared/query-keys.ts";
import { useUndoOffers } from "../../shared/use-optimistic.ts";
import { useApply, type AppliedProposal, type Proposal, type ProposedChange } from "./assistant.ts";
import type { CardChange, CardState } from "./ProposalCard.tsx";
import { canTakeBack, takeBack } from "./undo-change.ts";

/** What a member of a tool's input is, as text; none when it is not text. */
export function inputText(input: unknown, member: string): string | undefined {
  if (typeof input !== "object" || input === null) {
    return undefined;
  }
  const value: unknown = Reflect.get(input, member);
  return typeof value === "string" ? value : undefined;
}

/** A change of the API as the card draws it. */
export function cardChange(change: ProposedChange): CardChange {
  return { field: change.field, before: change.before, after: change.after };
}

/** Where the card stands: from what the person decided and what the server answered. */
function stateOf(
  dropped: boolean,
  undone: boolean,
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
    return { kind: undone ? "undone" : "saved" };
  }
  return applying === "error" ? { kind: "refused", problem: messageOf(error) } : { kind: "open" };
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
  const [offer, setOffer] = useState<number>();
  const live = useUndoOffers().some((each) => each.id === offer);
  const issue = inputText(proposal.input, "issue");
  const offerBack = (applied: AppliedProposal): void => {
    const saved = applied.changes.map((change) => cardChange(change));
    if (issue !== undefined && canTakeBack(proposal.tool, saved)) {
      setOffer(
        undoStore.offer({
          message: t("assistant.card.undone"),
          inline: true,
          undo: () => {
            setUndone(true);
            void takeBack(issue, saved);
          },
        }),
      );
    }
  };
  return {
    state: stateOf(dropped, undone, applying.status, applying.error),
    changes: (applying.data?.changes ?? proposal.changes).map((change) => cardChange(change)),
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
