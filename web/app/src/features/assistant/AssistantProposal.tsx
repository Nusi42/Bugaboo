import { t } from "@bugaboo/messages";

import type { Proposal } from "./assistant.ts";
import { ProposalCard } from "./ProposalCard.tsx";
import { inputText, useApplyProposal } from "./use-apply-proposal.ts";

/**
 * One proposal of the assistant as its card. The card draws it; applying and discarding are
 * `useApplyProposal`, so a card of another source (a standing rule) can be drawn the same way.
 */
export function AssistantProposal({ proposal }: { readonly proposal: Proposal }) {
  const control = useApplyProposal(proposal);
  const project = inputText(proposal.input, "project");
  return (
    <ProposalCard
      issue={inputText(proposal.input, "issue")}
      subject={project === undefined ? undefined : t("assistant.card.new-issue", { project })}
      changes={control.changes}
      state={control.state}
      onApply={control.apply}
      onDiscard={control.discard}
      onUndo={control.undo}
    />
  );
}
